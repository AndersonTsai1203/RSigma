mod spreadsheet;

use rsheet_lib::cell_value::CellValue;
use rsheet_lib::cells::column_number_to_name;
use rsheet_lib::command::Command;
use rsheet_lib::connect::{
    Connection, Manager, ReadMessageResult, Reader, WriteMessageResult, Writer,
};
use rsheet_lib::replies::Reply;

use std::error::Error;
use std::sync::Arc;
use std::thread;

use log::info;

use spreadsheet::Spreadsheet;

// Handle a single client connection in its own thread
fn handle_connection<R: Reader, W: Writer>(
    mut recv: R,
    mut send: W,
    spreadsheet: Arc<Spreadsheet>,
) -> Result<(), Box<dyn Error>> {
    loop {
        info!("Just got message");
        match recv.read_message() {
            ReadMessageResult::Message(msg) => {
                let reply = match msg.parse::<Command>() {
                    Ok(command) => match command {
                        Command::Get { cell_identifier } => {
                            let name = format!(
                                "{}{}",
                                column_number_to_name(cell_identifier.col),
                                cell_identifier.row + 1
                            );
                            let value = spreadsheet.get(&cell_identifier);
                            match value {
                                CellValue::Error(ref msg) if msg == "VariableDependsOnError" => {
                                    Reply::Error("Cell depends on another error cell".to_string())
                                }
                                _ => Reply::Value(name, value),
                            }
                        }
                        Command::Set {
                            cell_identifier,
                            cell_expr,
                        } => {
                            if let Err(e) = spreadsheet.set(cell_identifier, cell_expr) {
                                Reply::Error(format!("Error: {:?}", e))
                            } else {
                                continue;
                            }
                        }
                    },
                    Err(e) => Reply::Error(e),
                };

                match send.write_message(reply) {
                    WriteMessageResult::Ok => {}
                    WriteMessageResult::ConnectionClosed => break,
                    WriteMessageResult::Err(e) => return Err(Box::new(e)),
                }
            }
            ReadMessageResult::ConnectionClosed => break,
            ReadMessageResult::Err(e) => return Err(Box::new(e)),
        }
    }
    Ok(())
}

pub fn start_server<M>(mut manager: M) -> Result<(), Box<dyn Error>>
where
    M: Manager,
{
    // Create a new spreadsheet instance
    let spreadsheet = Arc::new(Spreadsheet::new());

    // Store handles to all spawned threads
    let mut handles = Vec::new();

    // Accept and handle connections until NoMoreConnections is received
    while let Connection::NewConnection { reader, writer } = manager.accept_new_connection() {
        let spreadsheet_clone = Arc::clone(&spreadsheet);

        let handle = thread::spawn(move || {
            if let Err(e) = handle_connection(reader, writer, spreadsheet_clone) {
                eprintln!("Connection error: {:?}", e);
            }
        });

        handles.push(handle);
    }

    // Wait for all connection threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}
