use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Instant;

use rsheet_lib::cell_expr::{CellArgument, CellExpr, CellExprEvalError};
use rsheet_lib::cell_value::CellValue;
use rsheet_lib::command::CellIdentifier;

/**
 * Represents a message type for the update worker thread
 * Used to communicate cell updates and shutdown signals
 */
#[derive(Clone, Debug)]
enum UpdateMessage {
    // Indicates a cell update
    CellUpdate {
        cell_id: CellIdentifier,
    },

    /// Signals the worker thread to shut down
    Shutdown,
}

/**
 * Stores information about a cell in the spreadsheet
 */
#[derive(Debug)]
pub struct CellInfo {
    value: CellValue,                    // Current value of the cell
    expression: String,                  // Original expression string
    dependencies: Vec<CellIdentifier>,   // Cells that this cell depends on
    dependents: HashSet<CellIdentifier>, // Cells that depend on this cell
    last_update_time: Instant,           // Timestamp of last successful update
}

/**
 * Main spreadsheet structure that manages cells and their relationships
 */
#[derive(Debug)]
pub struct Spreadsheet {
    cells: Arc<Mutex<HashMap<CellIdentifier, CellInfo>>>, // Thread-safe storage of cells
    update_sender: mpsc::Sender<UpdateMessage>,           // Channel for sending update messages
}

impl Spreadsheet {
    /**
     * HELPER FUNCTION
     * Creates a new spreadsheet instance
     *
     * Procedure:
     * 1. Creates thread-safe storage for cells using Arc and Mutex
     * 2. Sets up a channel for communication with worker thread
     * 3. Spawns worker thread to handle cell updates
     * 4. Returns configured spreadsheet instance
     */
    pub fn new() -> Self {
        let cells = Arc::new(Mutex::new(HashMap::new()));

        // Initialize channels for worker thread communication
        let (sender, receiver) = mpsc::channel();

        // Spawn worker thread to handle cell updates
        let worker_cells = Arc::clone(&cells);
        thread::spawn(move || {
            Self::process_cells_update(worker_cells, receiver);
        });

        Self {
            cells,
            update_sender: sender,
        }
    }

    /**
     * Public Function
     * Gets the value of a cell
     *
     * Procedure:
     * 1. Acquires lock on cells HashMap
     * 2. Checks if cell exists
     * 3. If cell exists:
     *    - Checks dependencies for errors
     *    - Returns error if any dependency has error
     *    - Otherwise returns cell value
     * 4. If cell doesn't exist, returns None
     */
    pub fn get(&self, cell_id: &CellIdentifier) -> CellValue {
        let cells = self.cells.lock().unwrap();
        if let Some(cell_info) = cells.get(cell_id) {
            // Check if any dependencies have errors
            for dep in &cell_info.dependencies {
                if let Some(dep_info) = cells.get(dep) {
                    if matches!(dep_info.value, CellValue::Error(_)) {
                        return CellValue::Error("VariableDependsOnError".into());
                    }
                }
            }
            cell_info.value.clone()
        } else {
            CellValue::None
        }
    }

    /**
     * Public Function
     * Sets a cell's value based on an expression
     *
     * Procedure:
     * 1. Records current timestamp
     * 2. Creates CellExpr from input string
     * 3. Extracts dependencies from expression
     * 4. Evaluates expression with current variable values
     * 5. Updates cell info with new value and dependencies
     * 6. Notifies worker thread of update
     */
    pub fn set(
        &self,
        cell_id: CellIdentifier,
        expression: String,
    ) -> Result<(), CellExprEvalError> {
        let current_time = Instant::now();
        let cell_expr = CellExpr::new(&expression);

        // Get all dependencies from the cell expression, including all cells within ranges
        let mut dependencies = Vec::new();
        for var_name in cell_expr.find_variable_names() {
            if !var_name.contains('_') {
                if let Ok(dep_id) = var_name.parse::<CellIdentifier>() {
                    dependencies.push(dep_id);
                }
            } else if let Some((start, end)) = Self::parse_range(&var_name) {
                // Add all cells in the range as dependencies
                for row in start.row..=end.row {
                    for col in start.col..=end.col {
                        dependencies.push(CellIdentifier { col, row });
                    }
                }
            }
        }

        // Resolve variables and evaluate expression
        let variables = self.resolve_variables(&cell_expr);
        let value = match cell_expr.evaluate(&variables) {
            Ok(value) => value,
            Err(CellExprEvalError::VariableDependsOnError) => {
                self.update_cell_info(
                    cell_id,
                    CellValue::Error("VariableDependsOnError".into()),
                    expression,
                    dependencies,
                    current_time,
                )?;
                return Ok(());
            }
        };

        // Update cell info and notify dependents
        self.update_cell_info(cell_id, value, expression, dependencies, current_time)?;
        Ok(())
    }

    /**
     * HELPER FUNCTION
     * Updates cell information and manages dependency relationships
     *
     * Procedure:
     * 1. Acquires lock on cells
     * 2. Collects old dependencies and dependents
     * 3. Removes cell from old dependencies' dependent lists
     * 4. Adds cell to new dependencies' dependent lists
     * 5. Updates/inserts cell info with new value
     * 6. Notifies worker thread of update
     */
    fn update_cell_info(
        &self,
        cell_id: CellIdentifier,
        value: CellValue,
        expression: String,
        dependencies: Vec<CellIdentifier>,
        current_time: Instant,
    ) -> Result<(), CellExprEvalError> {
        let mut cells = self.cells.lock().unwrap();

        // First collect the old dependencies and dependents
        let (old_dependencies, old_dependents) = if let Some(old_cell) = cells.get(&cell_id) {
            (old_cell.dependencies.clone(), old_cell.dependents.clone())
        } else {
            (Vec::new(), HashSet::new())
        };

        // Remove this cell from old dependencies' dependents lists
        for old_dep in old_dependencies {
            if let Some(dep_cell) = cells.get_mut(&old_dep) {
                dep_cell.dependents.remove(&cell_id);
            }
        }

        // Add this cell to new dependencies' dependents lists
        for dep in &dependencies {
            if let Some(dep_cell) = cells.get_mut(dep) {
                dep_cell.dependents.insert(cell_id);
            }
        }

        // Update/insert the cell info
        cells.insert(
            cell_id,
            CellInfo {
                value,
                expression,
                dependencies,
                dependents: old_dependents, // Preserve existing dependents
                last_update_time: current_time,
            },
        );

        // Notify single worker thread
        self.update_sender
            .send(UpdateMessage::CellUpdate { cell_id })
            .map_err(|_| CellExprEvalError::VariableDependsOnError)?;

        Ok(())
    }

    /**
     * HELPER FUNCTION
     * Resolves variables used in an expression
     *
     * Procedure:
     * 1. Creates empty variables HashMap
     * 2. For each variable name in expression:
     *    - If scalar (A1): gets single cell value
     *    - If range (A1_B2): gets vector or matrix of values
     * 3. Returns map of variable names to their values
     */
    fn resolve_variables(&self, cell_expr: &CellExpr) -> HashMap<String, CellArgument> {
        let mut variables: HashMap<String, CellArgument> = HashMap::new();

        for var_name in cell_expr.find_variable_names() {
            if var_name.contains('_') {
                // Handle range variables (vector or matrix)
                if let Some((start, end)) = Self::parse_range(&var_name) {
                    let arg = self.get_range_argument(&start, &end);
                    variables.insert(var_name.clone(), arg);
                }
            } else {
                // Handle scalar variables
                if let Ok(cell_id) = var_name.parse::<CellIdentifier>() {
                    let value = self.get(&cell_id);
                    variables.insert(var_name.clone(), CellArgument::Value(value));
                }
            }
        }

        variables
    }

    /**
     * HELP FUNCTION
     * Parses a range string into start and end cell identifiers
     *
     * Procedure:
     * 1. Splits string on underscore
     * 2. Parses first part as start cell
     * 3. Parses second part as end cell
     * 4. Returns tuple of (start, end) if valid
     */
    fn parse_range(range: &str) -> Option<(CellIdentifier, CellIdentifier)> {
        let parts: Vec<&str> = range.split('_').collect();
        if parts.len() != 2 {
            return None;
        }

        if let (Ok(start), Ok(end)) = (
            parts[0].parse::<CellIdentifier>(),
            parts[1].parse::<CellIdentifier>(),
        ) {
            Some((start, end))
        } else {
            None
        }
    }

    /**
     * HELPER FUNCTION
     * Converts a cell range into appropriate CellArgument type
     *
     * Procedure:
     * 1. Checks if any cells in range have errors
     * 2. Returns error if any found
     * 3. Determines range type (vertical/horizontal/matrix)
     * 4. Collects values into appropriate structure
     * 5. Returns vector or matrix argument
     */
    fn get_range_argument(&self, start: &CellIdentifier, end: &CellIdentifier) -> CellArgument {
        let cells = self.cells.lock().unwrap();

        // Check if any cells in the range have errors
        let has_errors = (start.row..=end.row).any(|row| {
            (start.col..=end.col).any(|col| {
                let cell_id = CellIdentifier { col, row };
                if let Some(cell) = cells.get(&cell_id) {
                    matches!(cell.value, CellValue::Error(_))
                } else {
                    false
                }
            })
        });

        if has_errors {
            return CellArgument::Value(CellValue::Error("VariableDependsOnError".into()));
        }
        drop(cells); // Release the lock before calling other functions

        if start.col == end.col {
            // Vertical vector
            self.get_vertical_vector(start, end)
        } else if start.row == end.row {
            // Horizontal vector
            self.get_horizontal_vector(start, end)
        } else {
            // Matrix
            self.get_matrix(start, end)
        }
    }

    /**
     * HELP FUNCTION
     * Get vertical vector from range
     *
     * Procedure:
     * 1. Creates vector to store values
     * 2. Iterates through rows at fixed column
     * 3. Gets value for each cell
     * 4. Returns vector as CellArgument
     */
    fn get_vertical_vector(&self, start: &CellIdentifier, end: &CellIdentifier) -> CellArgument {
        let values: Vec<CellValue> = (start.row..=end.row)
            .map(|row| {
                self.get(&CellIdentifier {
                    col: start.col,
                    row,
                })
            })
            .collect();
        CellArgument::Vector(values)
    }

    /**
     * HELP FUNCTION
     * Get horizontal vector from range
     *
     * Procedure:
     * 1. Creates vector to store values
     * 2. Iterates through columns at fixed row
     * 3. Gets value for each cell
     * 4. Returns vector as CellArgument
     */
    fn get_horizontal_vector(&self, start: &CellIdentifier, end: &CellIdentifier) -> CellArgument {
        let values: Vec<CellValue> = (start.col..=end.col)
            .map(|col| {
                self.get(&CellIdentifier {
                    col,
                    row: start.row,
                })
            })
            .collect();
        CellArgument::Vector(values)
    }

    /**
     * HELP FUNCTION
     * Get matrix from range
     *
     * Procedure:
     * 1. Creates nested vectors for matrix
     * 2. Iterates through rows
     * 3. For each row, iterates through columns
     * 4. Gets value for each cell
     * 5. Returns matrix as CellArgument
     */
    fn get_matrix(&self, start: &CellIdentifier, end: &CellIdentifier) -> CellArgument {
        let matrix: Vec<Vec<CellValue>> = (start.row..=end.row)
            .map(|row| {
                (start.col..=end.col)
                    .map(|col| self.get(&CellIdentifier { col, row }))
                    .collect()
            })
            .collect();
        CellArgument::Matrix(matrix)
    }

    /**
     * HELPER FUNCTION
     * Worker thread function that processes cell updates
     *
     * Procedure:
     * 1. Receives update messages from channel
     * 2. For each update:
     *    a. Builds dependency graph using BFS
     *    b. Performs topological sort of dependencies
     *    c. Updates cells in sorted order
     *    d. Handles timestamp ordering to prevent old updates overwriting new ones
     * 3. Continues until shutdown message received
     */
    fn process_cells_update(
        cells: Arc<Mutex<HashMap<CellIdentifier, CellInfo>>>,
        receiver: mpsc::Receiver<UpdateMessage>,
    ) {
        while let Ok(msg) = receiver.recv() {
            match msg {
                UpdateMessage::Shutdown => break,
                UpdateMessage::CellUpdate { cell_id } => {
                    // Step 1: Build dependency graph
                    let mut dependency_graph: HashMap<CellIdentifier, HashSet<CellIdentifier>> =
                        HashMap::new();
                    let mut to_process = VecDeque::new();
                    let mut discovered = HashSet::new();

                    // Initialize with the changed cell
                    to_process.push_back(cell_id);
                    discovered.insert(cell_id);

                    // Build complete dependency graph by doing a BFS
                    while let Some(current_id) = to_process.pop_front() {
                        let dependents = {
                            let cells_lock = cells.lock().unwrap();
                            cells_lock
                                .get(&current_id)
                                .map(|cell| cell.dependents.clone())
                                .unwrap_or_default()
                        };

                        for &dep_id in &dependents {
                            dependency_graph
                                .entry(dep_id)
                                .or_default()
                                .insert(current_id);

                            if discovered.insert(dep_id) {
                                to_process.push_back(dep_id);
                            }
                        }
                    }

                    // Step 2: Perform topological sort
                    let mut update_order = Vec::new();
                    let mut permanent_marks = HashSet::new();
                    let mut temporary_marks = HashSet::new();

                    // DFS-based topological sort
                    fn visit(
                        node: CellIdentifier,
                        graph: &HashMap<CellIdentifier, HashSet<CellIdentifier>>,
                        permanent_marks: &mut HashSet<CellIdentifier>,
                        temporary_marks: &mut HashSet<CellIdentifier>,
                        sorted: &mut Vec<CellIdentifier>,
                    ) {
                        // Skip if already fully processed
                        if permanent_marks.contains(&node) {
                            return;
                        }

                        // Check for cycles (should never happen in this application)
                        if temporary_marks.contains(&node) {
                            return;
                        }

                        // Mark temporarily for cycle detection
                        temporary_marks.insert(node);

                        // Visit all dependencies
                        if let Some(deps) = graph.get(&node) {
                            for &dep in deps {
                                visit(dep, graph, permanent_marks, temporary_marks, sorted);
                            }
                        }

                        // Remove temporary mark and add permanent mark
                        temporary_marks.remove(&node);
                        permanent_marks.insert(node);
                        sorted.push(node);
                    }

                    // Perform topological sort starting from all nodes
                    for &node in dependency_graph.keys() {
                        if !permanent_marks.contains(&node) {
                            visit(
                                node,
                                &dependency_graph,
                                &mut permanent_marks,
                                &mut temporary_marks,
                                &mut update_order,
                            );
                        }
                    }

                    // Step 3: Process cells in topologically sorted order
                    for cell_id in update_order {
                        let (expr, _deps) = {
                            let cells_lock = cells.lock().unwrap();
                            if let Some(cell) = cells_lock.get(&cell_id) {
                                (cell.expression.clone(), cell.dependencies.clone())
                            } else {
                                continue;
                            }
                        };

                        // Create cell expression evaluator
                        let cell_expr = CellExpr::new(&expr);

                        // Gather all required variables
                        let variables = {
                            let cells_lock = cells.lock().unwrap();
                            let mut vars = HashMap::new();

                            for var_name in cell_expr.find_variable_names() {
                                if !var_name.contains('_') {
                                    // Handle scalar variables
                                    if let Ok(var_id) = var_name.parse::<CellIdentifier>() {
                                        if let Some(cell) = cells_lock.get(&var_id) {
                                            vars.insert(
                                                var_name,
                                                CellArgument::Value(cell.value.clone()),
                                            );
                                        }
                                    }
                                } else if let Some((start, end)) = Self::parse_range(&var_name) {
                                    // Handle range variables
                                    let arg = if start.col == end.col {
                                        // Vertical vector
                                        let values: Vec<CellValue> = (start.row..=end.row)
                                            .map(|row| {
                                                let id = CellIdentifier {
                                                    col: start.col,
                                                    row,
                                                };
                                                cells_lock
                                                    .get(&id)
                                                    .map(|c| c.value.clone())
                                                    .unwrap_or(CellValue::None)
                                            })
                                            .collect();
                                        CellArgument::Vector(values)
                                    } else if start.row == end.row {
                                        // Horizontal vector
                                        let values: Vec<CellValue> = (start.col..=end.col)
                                            .map(|col| {
                                                let id = CellIdentifier {
                                                    col,
                                                    row: start.row,
                                                };
                                                cells_lock
                                                    .get(&id)
                                                    .map(|c| c.value.clone())
                                                    .unwrap_or(CellValue::None)
                                            })
                                            .collect();
                                        CellArgument::Vector(values)
                                    } else {
                                        // Matrix
                                        let matrix: Vec<Vec<CellValue>> = (start.row..=end.row)
                                            .map(|row| {
                                                (start.col..=end.col)
                                                    .map(|col| {
                                                        let id = CellIdentifier { col, row };
                                                        cells_lock
                                                            .get(&id)
                                                            .map(|c| c.value.clone())
                                                            .unwrap_or(CellValue::None)
                                                    })
                                                    .collect()
                                            })
                                            .collect();
                                        CellArgument::Matrix(matrix)
                                    };
                                    vars.insert(var_name, arg);
                                }
                            }
                            vars
                        };

                        // Evaluate cell with gathered variables
                        let current_time = Instant::now();
                        match cell_expr.evaluate(&variables) {
                            Ok(new_value) => {
                                let mut cells_lock = cells.lock().unwrap();
                                if let Some(cell) = cells_lock.get_mut(&cell_id) {
                                    // Only update if this evaluation is newer than the last update
                                    if current_time > cell.last_update_time {
                                        cell.value = new_value;
                                        cell.last_update_time = current_time;
                                    }
                                }
                            }
                            Err(CellExprEvalError::VariableDependsOnError) => {
                                let mut cells_lock = cells.lock().unwrap();
                                if let Some(cell) = cells_lock.get_mut(&cell_id) {
                                    if current_time > cell.last_update_time {
                                        cell.value =
                                            CellValue::Error("VariableDependsOnError".into());
                                        cell.last_update_time = current_time;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Drop for Spreadsheet {
    fn drop(&mut self) {
        // Send shutdown message to worker thread
        let _ = self.update_sender.send(UpdateMessage::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_basic_set_get() {
        let sheet = Spreadsheet::new();
        let cell = CellIdentifier { col: 0, row: 0 }; // A1

        assert!(sheet.set(cell, "42".to_string()).is_ok());
        sleep(Duration::from_millis(50)); // Allow time for processing
        assert_eq!(sheet.get(&cell), CellValue::Int(42));
    }

    #[test]
    fn test_sleep_then() {
        let sheet = Spreadsheet::new();
        let cell = CellIdentifier { col: 0, row: 0 }; // A1

        // Set A1 with a 500ms sleep, then value 5
        assert!(sheet.set(cell, "sleep_then(500, 5)".to_string()).is_ok());

        // Immediately set A1 with a 200ms sleep, then value 10
        assert!(sheet.set(cell, "sleep_then(200, 10)".to_string()).is_ok());

        // Wait for both operations to complete
        sleep(Duration::from_millis(700));

        // Should have value 10 since it was the more recent update
        assert_eq!(sheet.get(&cell), CellValue::Int(10));
    }

    #[test]
    fn test_dependencies() {
        let sheet = Spreadsheet::new();
        let a1 = CellIdentifier { col: 0, row: 0 };
        let b1 = CellIdentifier { col: 1, row: 0 };
        let c1 = CellIdentifier { col: 2, row: 0 };

        // Set up chain: C1 depends on B1 depends on A1
        assert!(sheet.set(a1, "5".to_string()).is_ok());
        assert!(sheet.set(b1, "A1 + 1".to_string()).is_ok());
        assert!(sheet.set(c1, "B1 * 2".to_string()).is_ok());

        sleep(Duration::from_millis(50)); // Allow time for processing

        assert_eq!(sheet.get(&a1), CellValue::Int(5));
        assert_eq!(sheet.get(&b1), CellValue::Int(6));
        assert_eq!(sheet.get(&c1), CellValue::Int(12));

        // Update A1 and verify cascade
        assert!(sheet.set(a1, "10".to_string()).is_ok());
        sleep(Duration::from_millis(100));

        assert_eq!(sheet.get(&a1), CellValue::Int(10));
        assert_eq!(sheet.get(&b1), CellValue::Int(11));
        assert_eq!(sheet.get(&c1), CellValue::Int(22));
    }

    #[test]
    fn test_vector_and_matrix() {
        let sheet = Spreadsheet::new();

        // Set up a 2x2 matrix
        assert!(sheet
            .set(CellIdentifier { col: 0, row: 0 }, "1".to_string())
            .is_ok()); // A1
        assert!(sheet
            .set(CellIdentifier { col: 1, row: 0 }, "2".to_string())
            .is_ok()); // B1
        assert!(sheet
            .set(CellIdentifier { col: 0, row: 1 }, "3".to_string())
            .is_ok()); // A2
        assert!(sheet
            .set(CellIdentifier { col: 1, row: 1 }, "4".to_string())
            .is_ok()); // B2

        // Test horizontal vector sum
        assert!(sheet
            .set(
                CellIdentifier { col: 2, row: 0 }, // C1
                "sum(A1_B1)".to_string()
            )
            .is_ok());

        // Test vertical vector sum
        assert!(sheet
            .set(
                CellIdentifier { col: 2, row: 1 }, // C2
                "sum(A1_A2)".to_string()
            )
            .is_ok());

        // Test matrix sum
        assert!(sheet
            .set(
                CellIdentifier { col: 2, row: 2 }, // C3
                "sum(A1_B2)".to_string()
            )
            .is_ok());

        sleep(Duration::from_millis(50));

        assert_eq!(
            sheet.get(&CellIdentifier { col: 2, row: 0 }),
            CellValue::Int(3)
        ); // C1 = 1 + 2
        assert_eq!(
            sheet.get(&CellIdentifier { col: 2, row: 1 }),
            CellValue::Int(4)
        ); // C2 = 1 + 3
        assert_eq!(
            sheet.get(&CellIdentifier { col: 2, row: 2 }),
            CellValue::Int(10)
        ); // C3 = 1 + 2 + 3 + 4
    }

    #[test]
    fn test_error_propagation() {
        let sheet = Spreadsheet::new();
        let a1 = CellIdentifier { col: 0, row: 0 };
        let b1 = CellIdentifier { col: 1, row: 0 };

        // Set A1 to an invalid expression
        assert!(sheet.set(a1, "invalid + expression".to_string()).is_ok());
        assert!(sheet.set(b1, "A1 + 1".to_string()).is_ok());

        sleep(Duration::from_millis(50));

        match sheet.get(&a1) {
            CellValue::Error(_) => (), // Expected
            other => panic!("Expected Error, got {:?}", other),
        }

        match sheet.get(&b1) {
            CellValue::Error(msg) if msg == "VariableDependsOnError" => (), // Expected
            other => panic!("Expected VariableDependsOnError, got {:?}", other),
        }
    }

    #[test]
    fn test_range_sum_with_updates() {
        let spreadsheet = Spreadsheet::new();

        // Initial setup: Set values for A1, B1, C1
        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 0 }, // A1
                "1".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 1, row: 0 }, // B1
                "2".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 2, row: 0 }, // C1
                "3".to_string(),
            )
            .unwrap();

        // Set D1 to sum of range A1_C1
        spreadsheet
            .set(
                CellIdentifier { col: 3, row: 0 }, // D1
                "sum(A1_C1)".to_string(),
            )
            .unwrap();

        // Check initial sum (should be 6)
        assert_eq!(
            spreadsheet.get(&CellIdentifier { col: 3, row: 0 }), // D1
            CellValue::Int(6)
        );

        // Update A1 to 4
        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 0 }, // A1
                "4".to_string(),
            )
            .unwrap();

        // Wait for updates to propagate
        thread::sleep(Duration::from_millis(2000));

        // Check sum after A1 update (should be 9)
        assert_eq!(
            spreadsheet.get(&CellIdentifier { col: 3, row: 0 }), // D1
            CellValue::Int(9)
        );

        // Update C1 to 10
        spreadsheet
            .set(
                CellIdentifier { col: 2, row: 0 }, // C1
                "10".to_string(),
            )
            .unwrap();

        // Wait for updates to propagate
        thread::sleep(Duration::from_millis(2000));

        // Check sum after C1 update (should be 16)
        assert_eq!(
            spreadsheet.get(&CellIdentifier { col: 3, row: 0 }), // D1
            CellValue::Int(16)
        );
    }

    #[test]
    fn test_range_sum_with_error() {
        let spreadsheet = Spreadsheet::new();

        // Set up initial values
        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 0 }, // A1
                "1".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 1, row: 0 }, // B1
                "invalid_expression".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 2, row: 0 }, // C1
                "3".to_string(),
            )
            .unwrap();

        // Set D1 to sum of range A1_C1
        spreadsheet
            .set(
                CellIdentifier { col: 3, row: 0 }, // D1
                "sum(A1_C1)".to_string(),
            )
            .unwrap();

        // D1 should have an error since B1 contains an invalid expression
        match spreadsheet.get(&CellIdentifier { col: 3, row: 0 }) {
            CellValue::Error(_) => (),
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_long_dependency_chain() {
        let spreadsheet = Spreadsheet::new();

        // Set up chain A1 -> A2 -> A3 -> A4 -> A5
        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 0 }, // A1
                "1".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 1 }, // A2
                "A1 + 1".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 2 }, // A3
                "A2 + 1".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 3 }, // A4
                "A3 + 1".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 4 }, // A5
                "A4 + 1".to_string(),
            )
            .unwrap();

        // Check initial value of A5
        assert_eq!(
            spreadsheet.get(&CellIdentifier { col: 0, row: 4 }), // A5
            CellValue::Int(5)                                    // 1 + 1 + 1 + 1 + 1 = 5
        );

        // Update A1 and check propagation
        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 0 }, // A1
                "7".to_string(),
            )
            .unwrap();

        thread::sleep(Duration::from_millis(2000));

        assert_eq!(
            spreadsheet.get(&CellIdentifier { col: 0, row: 4 }), // A5
            CellValue::Int(11)                                   // 7 + 1 + 1 + 1 + 1 = 11
        );
    }

    #[test]
    fn test_multi_level_dependency() {
        let spreadsheet = Spreadsheet::new();

        // Initial setup
        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 0 }, // A1
                "1".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 1 }, // A2
                "A1 + 1".to_string(),
            )
            .unwrap();

        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 2 }, // A3
                "A1 + A2 + 1".to_string(),
            )
            .unwrap();

        // Check initial value of A3
        assert_eq!(
            spreadsheet.get(&CellIdentifier { col: 0, row: 2 }), // A3
            CellValue::Int(4)                                    // 1 + 2 + 1 = 4
        );

        // Update A1 and check propagation
        spreadsheet
            .set(
                CellIdentifier { col: 0, row: 0 }, // A1
                "2".to_string(),
            )
            .unwrap();

        thread::sleep(Duration::from_millis(2000));

        assert_eq!(
            spreadsheet.get(&CellIdentifier { col: 0, row: 2 }), // A3
            CellValue::Int(6)                                    // 2 + 3 + 1 = 6
        );
    }
}
