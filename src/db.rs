use std::{collections::HashMap, ops::Deref};

use rusqlite::{Connection, ffi::SQLITE_LAST_ERRNO};

pub struct Db {
    filename: String,
    conn: Connection,
    entries: Vec<Entry>,
}

#[derive(Clone)]
struct Tag {
    id: u32,
    tag: String,
}

#[derive(Clone)]
pub struct Entry {
    id: u32,
    created_time: u32,
    updated_time: u32,
    title: String,
    content: String,
    tags: Option<Vec<Tag>>,
}

impl Entry {
    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }
}

impl Db {
    pub fn new(filename: &str) -> Self {
        Self {
            filename: filename.to_string(),
            conn: Connection::open(filename).unwrap(),
            entries: Vec::new(),
        }
    }

    pub fn get_entries(&self) -> Vec<Entry> {
        self.entries.clone()
    }

    pub fn initialize_db(&self) -> Result<(), Box<dyn std::error::Error>> {
        // let conn = Connection::open(&self.filename)?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS entries (
                entry_id INTEGER NOT NULL PRIMARY KEY,
                entry_created_time timestamp default (strftime('%s', 'now')),
                entry_updated_time timestamp default (strftime('%s', 'now')),
                entry_title TEXT,
                entry_content TEXT
            )",
            (),
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                tag_id INTEGER NOT NULL PRIMARY KEY,
                tag TEXT,
                UNIQUE(tag)
            )",
            (),
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS entry_tags (
                entry_id INTEGER,
                tag_id INTEGER,
                FOREIGN KEY(entry_id) REFERENCES entries(entry_id),
                FOREIGN KEY(tag_id) REFERENCES tags(tag_id),
                UNIQUE(entry_id, tag_id)
            )",
            (),
        )?;
        self.conn.execute(
            "CREATE VIEW IF NOT EXISTS entries_w_tags AS SELECT entries.entry_id, entry_created_time, entry_updated_time, entry_title, 
                    entry_content, group_concat(tags.tag_id, ':') AS tags
                FROM
                    (entries JOIN entry_tags ON entries.entry_id = entry_tags.entry_id)
                    JOIN tags ON entry_Tags.tag_id = tags.tag_id
                GROUP BY entries.entry_id;
            ",
            (),
        )?;
        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS update_updated_time UPDATE OF entry_title, entry_content ON entries
            BEGIN
                UPDATE entries SET entry_updated_time=strftime('%s', 'now') WHERE entry_id = entry_id;
            END;",
            (),
        )?;
        Ok(())
    }
    
    fn get_tags(&self) -> Result<HashMap<u32,Tag>, rusqlite::Error> {
        let mut tags = HashMap::new();
        //let conn = Connection::open(&self.filename)?;
        let mut stmt = self.conn.prepare(
            "SELECT tag_id, tag FROM tags;",
        )?;
        let results = stmt.query_map((), |row| {
            Ok(Tag {
                id: row.get(0)?,
                tag: row.get(1)?,
            })
        })?;
    
        for t in results {
            if let Ok(tag) = t {
                tags.insert(tag.id, tag);
            }
        }
    
        Ok(tags)
    }
    
    pub fn update_entries(&mut self) -> Result<(), rusqlite::Error> {
        let tags = self.get_tags()?;
        let mut entries = Vec::new();
        // let conn = Connection::open(&self.filename)?;
        let mut stmt = self.conn.prepare(
            "SELECT * FROM entries_w_tags"
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
                id: row.get(0)?,
                created_time: row.get(1)?,
                updated_time: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                tags: entry_tags
            })
        })?;
        for r in results {
            if let Ok(entry) = r {
                entries.push(entry);
            }
        }
        self.entries = entries;
        Ok(())
        
    }

    fn create_tag(&mut self, tag:&str) -> Result<u32, rusqlite::Error> {
        // let conn = Connection::open(&self.filename)?;
        let mut stmt = self.conn.prepare("SELECT tag_id FROM tags where tag = ?1")?;
        match stmt.query_row([&tag], |r| r.get(0)) {
            Ok(id) => Ok(id),
            _ => {
                self.conn.execute(
                    "INSERT INTO tags (tag) VALUES (?1)",
                    (&tag,)
                )?;
                Ok(self.conn.last_insert_rowid() as u32)
            },
        }
    }

    fn create_entry(&mut self, entry_title: &str, entry_content: &str, tags: Option<Vec<String>>) 
            -> Result<(), rusqlite::Error> {
        // let conn = Connection::open(&self.filename)?;
        self.conn.execute(
            "INSERT INTO entries (entry_title, entry_content)
            VALUES (?1, ?2)",
            (&entry_title, &entry_content),
        )?;
        let entry_id = self.conn.last_insert_rowid();
        if let Some(tvec) = tags {
            for tag in tvec {
                // self.conn.execute(
                //     "INSERT INTO tags (tag) values (?1)",
                //     (&tag,),
                // )?;
                // let tag_id = self.conn.last_insert_rowid();
                let tag_id = self.create_tag(&tag)?;
                self.conn.execute(
                    "INSERT INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                    (&entry_id, &tag_id)
                )?;
            }
        }
        self.update_entries()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn basic_db_function() {
        let file = "test.db";
        // match fs::remove_file(file) {
        //     Ok(()) => println!("removed {}", file),
        //     Err(e) => println!("{}", e),
        // }
        let mut db = Db::new(file);
        db.initialize_db().unwrap();
        let title = "Test1";
        let content = "Test content";
        let tags = Some(vec!["foo".to_string(), "bar".to_string()]);
        db.create_entry(title, content, tags.clone()).unwrap();
        db.update_entries().unwrap();
        assert_eq!(db.entries[0].title, title);
        assert_eq!(db.entries[0].content, content);
        assert_eq!(
            db.entries[0].tags.as_ref().unwrap().iter().map(|t| t.clone().tag).collect::<Vec<String>>(), 
            tags.unwrap()
        );
        db.conn.close().unwrap();
        
    }

    #[test]
    fn test_insert_tag() {
        let file = "test2.db";
        // match fs::remove_file(file) {
        //     Ok(()) => println!("removed {}", file),
        //     Err(e) => println!("{}", e),
        // }
        let mut db = Db::new(file);
        db.initialize_db().unwrap();
        let x = db.create_tag("foo").unwrap();
        assert_eq!(x, 1);
        let y = db.create_tag("bar").unwrap();
        assert_eq!(y, 2);
        // let t: u32 = db.conn.query_row("select tag_id from tags where tag = '?1'", ("foo",), |r| r.get(0)).unwrap();
        // assert_eq!(t,1);
        let z = db.create_tag("foo").unwrap();
        assert_eq!(z, 1);
        db.conn.close().unwrap();
    }
}