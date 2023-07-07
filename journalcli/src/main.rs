use std::error::Error;

use clap::{command, Command, arg};
use dialoguer::{Input, Editor};
use journaldb::{Tag, Entry, Db};

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
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents 'None'"),
    }?;
    Ok(())
}

fn create_journal_entry(db: &mut Db) -> Result<(), Box<dyn Error>> {
    let title: String = Input::new()
        .with_prompt("Enter entry_title")
        .with_initial_text(format!("{} Entry", chrono::offset::Local::now().format("%d-%m-%Y")))
        .interact_text()?;

    if let Some(content) = Editor::new().edit("Enter entry content")? {
        let tags: Vec<Tag> = Input::<String>::new()
            .with_prompt("Enter tags separated by comma")
            .interact_text()?
            .split(',')
            .map(|t| {
                Tag::new(String::from(t))
            }).collect();
        let tags = if tags.len() > 0 {
            Some(tags)
        }    
        else {
            None
        };

        let mut entry = Entry::new(
            title,
            content,
            tags,
        );

        db.create_entry(&mut entry)?;
    } 

    Ok(())

}

fn print_journal_entries(db: &mut Db) -> Result<(), Box<dyn Error>> {
    let entries = db.get_entries();
    for entry in entries {
        println!("{} - {}", entry.get_id(), entry.get_title());
    }

    Ok(())
}

fn delete_journal_entry(db: &mut Db, entry_id: u32) -> Result<(), Box<dyn Error>> {
    if let Some(entry ) = db.get_entry_by_id(entry_id) {
        db.delete_entry(& entry)?;
        println!("Entry [{} - {}] deleted", entry.get_id(), entry.get_title());
    }
    else {
        println!("Entry with id {} not found", entry_id);
    }
    Ok(())
}