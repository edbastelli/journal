use std::error::Error;

use clap::{command, Command, arg};

use journaldb::{Db};

mod util;
use crate::util::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut db = Db::new("journal.db");
    db.initialize_db()?;
    db.update_entries()?;

    let matches = command!()
    .propagate_version(true)
    .subcommand_required(true)
    .subcommand(
        Command::new("create")
            .about("Create new Entry"),
    )
    .subcommand(
        Command::new("list")
            .about("List all Entries"),
    )
    .subcommand(
        Command::new("delete")
            .about("Delete an Entry")
            .arg(arg!([entry_id])),
    )
    .subcommand(
        Command::new("show")
            .about("Show journal entry"),
    )
    .subcommand(
        Command::new("edit")
            .about("Edit journal entry"),
    )
    .get_matches();
    match matches.subcommand() {
        Some(("create", _)) => create_journal_entry(&mut db),
        Some(("list", _)) => print_journal_entries(&mut db),
        Some(("delete", args)) => Ok({
            if let Some(x) = args.get_one::<String>("entry_id") {
                if let Ok(entry_id) = x.parse::<u32>() {
                    delete_journal_entry(&mut db, entry_id)?;
                }
                else {
                    println!("Entry id must be a number");
                }
            }
        }),
        Some(("show", _)) => show_journal_entry(&db),
        Some(("edit", _)) => edit_journal_entry(&mut db),
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents 'None'"),
    }?;
    Ok(())
}

