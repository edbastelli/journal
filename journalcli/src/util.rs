use std::error::Error;

use dialoguer::{Input, Editor, Select, theme::ColorfulTheme, console::Term};
use journaldb::{Tag, Entry, Db};

pub fn create_journal_entry(db: &mut Db) -> Result<(), Box<dyn Error>> {
    let title: String = Input::new()
        .with_prompt("Enter entry_title")
        .with_initial_text(format!("{} Entry", chrono::offset::Local::now().format("%d-%m-%Y")))
        .interact_text()?;

    if let Some(content) = Editor::new().edit("Enter entry content")? {
        // let tags: Vec<Tag> = Input::<String>::new()
        //     .with_prompt("Enter tags separated by comma")
        //     .allow_empty(true)
        //     .interact_text()?
        //     .split(',')
        //     .map(|t| {
        //         Tag::new(String::from(t))
        //     }).collect();
        // let tags = if tags.len() > 0 {
        //     Some(tags)
        // }    
        // else {
        //     None
        // };

        let tags: Option<Vec<Tag>> = Input::<String>::new()
            .with_prompt("Enter tags separated by comma")
            .allow_empty(true)
            .interact_text()?
            .split(',')
            .map(|t| {
                match t.len() {
                    0 => None,
                    _ => Some(Tag::new(String::from(t))),
                }
            }).collect();
        println!("{:#?}", tags);

        let mut entry = Entry::new(
            title,
            content,
            tags,
        );

        db.create_entry(&mut entry)?;
    } 

    Ok(())

}

pub fn print_journal_entries(db: &mut Db) -> Result<(), Box<dyn Error>> {
    let entries = db.get_entries();
    for entry in entries {
        println!("{} - {}", entry.get_id(), entry.get_title());
    }

    Ok(())
}

pub fn delete_journal_entry(db: &mut Db, entry_id: u32) -> Result<(), Box<dyn Error>> {
    if let Some(entry ) = db.get_entry_by_id(entry_id) {
        db.delete_entry(& entry)?;
        println!("Entry [{} - {}] deleted", entry.get_id(), entry.get_title());
    }
    else {
        println!("Entry with id {} not found", entry_id);
    }
    Ok(())
}

pub fn show_journal_entry(db: &Db) -> Result<(), Box<dyn Error>> {
    let entries = db.get_entries();
    let items = &entries
        .iter()
        .map(|e| e.get_title())
        .collect::<Vec<String>>();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        // Some(index) => println!("{}", entries[index].get_content()),
        Some(index) => print!(
            "Title:\n{}\n\nContent:\n{}\n\nTags:\n{}\n\nCreated:\n{}\nUpdated:\n{}\n",
            entries[index].get_title(),
            entries[index].get_content(),
            entries[index]
                .get_tags()
                .unwrap_or(vec![])
                .iter()
                .map(|t| t.get_tag())
                .collect::<Vec<String>>()
                .join(","),
            entries[index].get_created_time(),
            entries[index].get_updated_time(),
        ),
        None => println!("None selected"),
    }
    Ok(())
}

pub fn edit_journal_entry(db: &mut Db) -> Result<(), Box<dyn Error>> {
    let entries = db.get_entries();
    let items = &entries
        .iter()
        .map(|e| e.get_title())
        .collect::<Vec<String>>();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .interact_on_opt(&Term::stderr())
        .unwrap()
        .unwrap();
    let title: String = Input::new()
        .with_prompt("Enter entry_title")
        .with_initial_text(format!("{}", &entries[selection].get_title()))
        .interact_text()?;
    let mut content = entries[selection].get_content();
    if let Ok(Some(c)) = Editor::new()
        .edit(&format!("{}", &content))
        {
            content = c;}
    else { }
    let tags: Option<Vec<Tag>> = Input::<String>::new()
        .with_prompt("Enter tags separated by comma")
        .with_initial_text(format!("{}", entries[selection]
            .get_tags()
            // .unwrap_or(vec![Tag::new("".to_string())])
            .unwrap_or(vec![])
            .iter()
            .map(|t| t.get_tag()).collect::<Vec<String>>().join(",")))
        .allow_empty(true)
        .interact_text()?
        .split(',')
        .map(|t| {
            match t.len() {
                0 => None,
                _ => Some(Tag::new(String::from(t))),
            }
        }).collect();
    println!("{:#?}", tags);
    let mut entry = entries[selection].clone();
    entry.set_title(title);
    entry.set_content(content);
    entry.set_tags(tags);
    db.edit_entry(&mut entry)?;
    Ok(())
}