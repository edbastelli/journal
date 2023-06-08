

use crossterm::event::{Event, self, KeyCode};
use rusqlite::Connection;
use tui::{backend::Backend, Terminal, widgets::ListState};
use std::{collections::HashMap, ops::Deref};

use crate::ui::ui;


pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Clone)]
pub struct Tag {
    tag_id: u32,
    tag: String,
}

#[derive(Clone)]
pub struct Entry {
    entry_id: u32,
    entry_created_time: u32,
    entry_updated_time: u32,
    entry_title: String,
    entry_content: String,
    entry_tags: Option<Vec<Tag>>,
}

impl Entry {
    pub fn get_id(self) -> u32 {
        self.entry_id
    }
    pub fn get_title(self) -> String {
        self.entry_title
    }
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn unselect(&mut self) {
        self.state.select(None);
    }
}

/// App holds the state of the application
pub struct App {
    /// Current value of the input box
    pub input: String,
    /// Current input mode
    pub input_mode: InputMode,
    /// History of recorded messages
    pub messages: Vec<String>,
    // pub entries: Vec<Entry>,
    pub entries: Option<StatefulList<Entry>>,
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            entries: match get_entries("journal.db") {
                Ok(entries) => Some(StatefulList::with_items(entries)),
                Err(_) => None,
            },
        }
    }
}

pub fn initialize_db(dbfile: &str) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(dbfile)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entries (
            entry_id INTEGER NOT NULL PRIMARY KEY,
            entry_created_time timestamp default (strftime('%s', 'now')),
            entry_updated_time timestamp default (strftime('%s', 'now')),
            entry_title TEXT,
            entry_content TEXT
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
            tag_id INTEGER NOT NULL PRIMARY KEY,
            tag TEXT,
            UNIQUE(tag)
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entry_tags (
            entry_id INTEGER,
            tag_id INTEGER,
            FOREIGN KEY(entry_id) REFERENCES entries(entry_id),
            FOREIGN KEY(tag_id) REFERENCES tags(tag_id),
            UNIQUE(entry_id, tag_id)
        )",
        (),
    )?;
    conn.execute(
        "CREATE VIEW entries_w_tags AS SELECT entries.entry_id, entry_created_time, entry_title, 
                entry_content, group_concat(tag, \", \") AS tags
            FROM
                (entries JOIN entry_tags ON entries.entry_id = entry_tags.entry_id)
                JOIN tags ON entry_Tags.tag_id = tags.tag_id
            GROUP BY entries.entry_id;
        ",
        (),
    )?;
    conn.execute(
        "CREATE TRIGGER update_updated_time UPDATE OF entry_title, entry_content ON entries
        BEGIN
            UPDATE entries SET entry_updated_time=strftime('%s', 'now') WHERE entry_id = entry_id;
        END;",
        (),
    )?;
    Ok(())
}

fn get_tags(dbfile: &str) -> Result<HashMap<u32,Tag>, Box<dyn std::error::Error>> {
    let mut tags = HashMap::new();
    let conn = Connection::open(dbfile)?;
    let mut stmt = conn.prepare(
        "SELECT tag_id, tag FROM tags;",
    )?;
    let results = stmt.query_map((), |row| {
        Ok(Tag {
            tag_id: row.get(0)?,
            tag: row.get(1)?,
        })
    })?;

    for t in results {
        if let Ok(tag) = t {
            tags.insert(tag.tag_id, tag);
        }
    }

    Ok(tags)
}

pub fn get_entries(dbfile: &str) -> Result<Vec<Entry>, Box<dyn std::error::Error>> {
    let tags = get_tags(dbfile)?;
    let mut entries = Vec::new();
    let conn = Connection::open(dbfile)?;
    let mut stmt = conn.prepare(
        "SELECT entries.entry_id, entry_created_time, entry_updated_time, entry_title, entry_content, group_concat(tags.tag_id, ':') AS tags
                FROM (entries JOIN entry_tags ON entries.entry_id = entry_tags.entry_id)
                    JOIN tags ON entry_tags.tag_id = tags.tag_id
                GROUP BY entries.entry_id
                ORDER BY entry_created_time DESC"
    )?;
    let results = stmt.query_map((), |row| {
        let entry_tags_db: String = row.get(5)?;
        let entry_tags = entry_tags_db.split(':').map(|x| {
            if let Ok(tag_id) = x.parse() {
                let tag = tags.get(&tag_id).unwrap().deref().clone();
                Some(tag)
            }
            else {
                None
            }
        }).collect::<Option<Vec<Tag>>>();

        Ok(Entry {
            entry_id: row.get(0)?,
            entry_created_time: row.get(1)?,
            entry_updated_time: row.get(2)?,
            entry_title: row.get(3)?,
            entry_content: row.get(4)?,
            entry_tags: entry_tags
        })
    })?;
    for r in results {
        if let Ok(entry) = r {
            entries.push(entry);
        }
    }
    Ok(entries)
    
}


pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    let dbfile = "journal.db";
    initialize_db(dbfile)?;
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('e') => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Enter => {
                        app.messages.push(app.input.drain(..).collect());
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    _ => {}
                },
            }
        }
    }
}

fn create_entry(dbfile: &str, entry_title: &str, entry_content: &str, tags: Option<Vec<String>>) -> Result<(), rusqlite::Error> {
    let conn = Connection::open(dbfile)?;
    conn.execute(
        "INSERT INTO entries (entry_title, entry_content)
        VALUES (?1, ?2)",
        (&entry_title, &entry_content),
    )?;
    let entry_id = conn.last_insert_rowid();
    if let Some(tvec) = tags {
        for tag in tvec {
            conn.execute(
                "INSERT INTO tags (tag) values (?1)",
                (&tag,),
            )?;
            let tag_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                (&entry_id, &tag_id)
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn basic_db_function() {
        match fs::remove_file("test.db") {
            Ok(()) => println!("removed test.db"),
            Err(e) => println!("{}", e),
        }
        initialize_db("test.db").unwrap();
        let title = "Test1";
        let content = "Test content";
        let tags = Some(vec!["foo".to_string(), "bar".to_string()]);
        create_entry("test.db", title, content, tags.clone()).unwrap();
        let entries = get_entries("test.db").unwrap();
        assert_eq!(entries[0].entry_title, title);
        assert_eq!(entries[0].entry_content, content);
        assert_eq!(entries[0].entry_tags.as_ref().unwrap().iter().map(|t| t.clone().tag).collect::<Vec<String>>(), tags.unwrap());
        
    }
}