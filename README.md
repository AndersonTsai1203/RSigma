# 🧮 RSigma 
Mixes **"Rust"** and the Greek letter **Sigma (∑)**, often associated with summation/spreadsheets. Feels like something you'd expect in a research lab.

## 📘 Background & Motivation
**RSigma** is a **lightweight**, **high-performance spreadsheet computation engine** written in **Rust**, designed to explore reactive dataflow, concurrency, and safe system architecture. Inspired by the legacy of early "killer apps" like VisiCalc, RSigma reimagines the spreadsheet not as a GUI-bound tool, but as a **server-controlled**, **reactive data platform** — ideal for integration, automation, and scalable computation.

Instead of focusing on traditional UI-driven interactivity, RSigma offers a **command-driven interface** — allowing programmatic cell manipulation, dynamic formula updates, and real-time recalculations—all under a thread-safe, concurrent architecture.

## 💡 Why RSigma
**RSigma** is designed to bring a fully featured, multi-threaded spreadsheet manager capable of handling complex dependencies and cell evaluations in real-time. The spreadsheet manager makes it easy to interact with data through commands such as ```get``` and ```set```, and supports advanced features such as:
- **Real-time data updates** via multi-threaded operations, ensuring atomic transactions.
- **Variable and expression handling** for a flexible and dynamic spreadsheet experience.
- **Cell Dependency Management** that efficiently updates dependent cells asynchronously and handles chains of dependencies.

**RSigma** fills the gap for spreadsheet operations that need to handle complex formulae, multiple users, and concurrent calculations, all while providing a robust and scalable architecture suited for real-world applications like collaborative data analysis or task tracking.

## 🎯 Project Goals
The main goals of RSigma are:
- **Real-Time Interaction** – Ensure that the user can interact with the spreadsheet in real-time, with atomic updates and responses.
- **Concurrency Handling** – Support multiple concurrent users/commands with thread-safe operations while ensuring that each user's session is properly isolated.
- **Formula Evaluation** – Implement a formula language (based on Rhai) that allows for complex calculations and interactions between cells, handling both basic operations and advanced features such as summing ranges.
- **Error Propagation** – Ensure that errors in cell dependencies are managed correctly and reported to users appropriately.
- **Asynchronous Dependency Updates** – Properly handle dependent cells and allow updates to propagate asynchronously while maintaining consistency.

## 🚀 Project Structure
```text
RSigma/
│
├── src/                        # Core source directory
│   ├── core.rs                 # Core functionality of the spreadsheet manager
│   ├── commands.rs             # Command handling (parsing, get, set operations)
│   ├── cell_expr.rs            # Cell expression handling and formula evaluation
│   ├── dependency.rs           # Dependency management between cells
│   ├── concurrency.rs          # Multi-threading and concurrency logic
│   ├── errors.rs               # Error handling and propagation
│   └── main.rs                 # Entry point of the application
│
├── scripts/                    # Optional directory for useful scripts
│   ├── cli.rs                  # Command-line interface logic for interacting with the spreadsheet
│   └── init.rs                 # Initialization script to bootstrap the spreadsheet system (e.g., default state, test data)
│
├── tests/                      # Unit and integration tests
│   ├── core_tests.rs           # Tests for core functionality
│   ├── command_tests.rs        # Tests for get/set operations
│   ├── cell_expr_tests.rs      # Tests for cell expressions and formulas
│   ├── dependency_tests.rs     # Tests for dependency resolution
│   ├── concurrency_tests.rs    # Tests for multi-threading and concurrency
│   └── error_tests.rs          # Tests for error handling and validation
│
├── examples/                   # Examples demonstrating usage of the spreadsheet system
│   ├── simple_example.rs       # Simple example with basic get/set commands
│   ├── formula_example.rs      # Example for complex formula handling
│   └── concurrency_example.rs  # Example showcasing multi-threading and concurrent users
│
├── assets/                     # Optional directory for assets (e.g., documentation, images)
│   └── README.md               # Project overview and documentation
│
├── Cargo.toml                  # Cargo configuration file (for dependencies, build configuration)
└── README.md                   # Project documentation
```

### 🧩 Module Breakdown
#### 1. Core Module (```core.rs```)
- Purpose: This module acts as the central hub of the RSigma system. It manages the primary logic of the spreadsheet, including storage of cell values, handling updates, and maintaining the overall state of the spreadsheet.
- Responsibilities:
  - Storing cell data and related metadata.
  - Handling updates when cells are modified or referenced.
  - Managing the consistency of the spreadsheet’s state.

#### 2. Command Handling Module (```commands.rs```)
- Purpose: This module is responsible for parsing and handling commands like get, set, and other spreadsheet-related operations.
- Responsibilities:
  - Parsing and interpreting user input.
  - Executing commands (e.g., setting a cell value, retrieving a value).
  - Validating commands and providing meaningful feedback or errors.

#### 3. Cell Expression Module (```cell_expr.rs```)
- Purpose: This module provides the functionality to handle formulas and cell expressions within the spreadsheet.
- Responsibilities:
  - Parsing and evaluating expressions (e.g., formulas such as =A1 + B2).
  - Handling complex expressions involving multiple cells.
  - Updating dependent cells when the result of a formula changes.

#### 4. Dependency Management Module (```dependency.rs```)
- Purpose: Manages the dependencies between cells (i.e., tracking which cells depend on which).
- Responsibilities:
  - Keeping track of cells that depend on the value of other cells.
  - Ensuring proper updates are triggered when a dependent cell's value changes.
  - Maintaining a dependency graph to optimize re-calculations.

#### 5. Concurrency Module (```concurrency.rs```)
- Purpose: Handles the multi-threaded aspects of the spreadsheet system, allowing for concurrent interactions and operations.
- Responsibilities:
  - Ensuring thread safety when handling user requests.
  - Managing concurrent read and write operations without conflicts.
  - Optimizing performance in a multi-user environment.

#### 6. Error Handling Module (```errors.rs```)
- Purpose: Defines the custom error types used throughout the RSigma system and manages error handling.
- Responsibilities:
  - Defining custom errors for specific failure cases (e.g., invalid formula, cell not found).
  - Centralizing error management for easy debugging and clearer feedback to users.

## 🧠 Core Features
#### 1. Spreadsheet Initialization
- [ ] Define the spreadsheet data structure (rows, columns, and cell storage).
- [ ] Implement initialization logic to load an empty or pre-existing spreadsheet.
- [ ] Create functions for cell access (get/set values).
- [ ] Add basic error handling for invalid cell references.

#### 2. Command Parsing & Execution
- [ ] Implement command input handler (e.g., for get and set commands).
- [ ] Parse commands correctly to interpret cell references and formulas.
- [ ] Add logic to execute commands based on parsed data.
- [ ] Implement error feedback for invalid commands (e.g., unknown cells, bad syntax).
- [ ] Create unit tests for command execution scenarios.

#### 3. Cell Expression Handling
- [ ] Define expression syntax rules (e.g., A1 + B2, SUM(A1:B3)).
- [ ] Implement expression parser to identify operators and operands.
- [ ] Develop evaluation logic to compute results for simple expressions.
- [ ] Handle circular references and prevent infinite loops in formulas.
- [ ] Add dependency tracking for cells involved in expressions.
- [ ] Optimize re-calculation when dependent cells change.

#### 4. Dependency Management
- [ ] Design a dependency tracking system (graph of cell dependencies).
- [ ] Implement logic to track dependencies when cells contain formulas that refer to other cells.
- [ ] Ensure proper updates are triggered when a value in a dependent cell changes.
- [ ] Optimize recalculations to minimize redundant evaluations.
- [ ] Add unit tests to ensure accurate dependency management.

#### 5. Concurrency Handling
- [ ] Integrate multi-threading support for concurrent access to the spreadsheet.
- [ ] Ensure thread safety during read/write operations on cells.
- [ ] Optimize concurrency logic for scenarios where multiple users or operations access the same cell.
- [ ] Manage read/write conflicts by introducing locking mechanisms or atomic operations.
- [ ] Test concurrency features with simultaneous user inputs.

#### 6. Error Handling
- [ ] Define custom error types (e.g., invalid cell, circular reference, syntax errors).
- [ ] Implement robust error messages for the user in the command interface.
- [ ] Centralize error handling logic to ensure consistency across the system.
- [ ] Log errors for debugging purposes.
- [ ] Add unit tests to verify that errors are triggered and handled correctly.

#### 7. Persistence (Optional for Future)
- [ ] Implement persistence layer to save and load the state of the spreadsheet to/from a file (e.g., JSON or CSV).
- [ ] Ensure that formulas and values are preserved during save/load.
- [ ] Implement recovery logic in case of data corruption or crash.
- [ ] Test saving/loading functionality with large datasets.

#### 8. Performance Optimization (Optional for Future)
- [ ] Profile the application to identify performance bottlenecks.
- [ ] Optimize memory usage for large spreadsheets or complex expressions.
- [ ] Improve recalculation speed for large sets of dependent cells.
- [ ] Use caching strategies for frequently accessed or static data.

### Extension Ideas (Optional)
- [ ] Data Import/Export: Adding the ability to import/export spreadsheet data in popular formats such as CSV, Excel, or JSON.
- [ ] Graphing/Visualization: Adding support for simple chart generation based on cell values, making the spreadsheet more interactive
- [ ] Undo/Redo Functionality: Implement a history of changes to support undo/redo operations, allowing users to backtrack if needed.
- [ ] Collaborative Features: Allow multiple users to edit the same spreadsheet and automatically sync changes, similar to Google Sheets.
- [ ] Real-Time Collaboration: Implement a feature where changes made by one user immediately reflect in the spreadsheets of other users.

## 🔧 Extended Features (Optional - Todo)
#### 1. WebAssembly-based UI Frontend
Build a minimal WASM UI to visualize and interact with the spreadsheet in-browser.
#### 2. Persistence Layer
Support saving/loading spreadsheet state to disk via JSON, SQLite, or flat files.
#### 3. Formula Language Enhancements
Add support for functions like SUM(), AVG(), IF(), MAX() to simulate Excel-like behavior.
#### 4. Multiclient Support
Enable multiple clients to connect simultaneously to the server and perform safe, concurrent edits.
#### 5. RESTful API or gRPC
Expose spreadsheet operations via a web interface for integration into external systems or microservices.
#### 6. Versioning and Undo
Implement a history system to allow reverting or auditing changes made to the spreadsheet.








