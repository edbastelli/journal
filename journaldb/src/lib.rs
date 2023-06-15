use std::{collections::HashMap, ops::Deref};

use rusqlite::{Connection};

pub struct Db {
    filename: String,
    conn: Connection,
    entries: Vec<Entry>,
}

#[derive(Clone)]
pub struct Tag {
    id: u32,
    tag: String,
}

impl Tag {
    pub fn new(tag: String) -> Self {
        Tag {
            id: 0,
            tag
        }
    }
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

    pub fn new(title: String, content: String, tags: Option<Vec<Tag>>) -> Self {
        Entry {
            id: 0,
            created_time: 0,
            updated_time: 0,
            title,
            content,
            tags
        }
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
        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS delete_deleted_entry_tags
            AFTER DELETE ON entries
            FOR EACH ROW
            BEGIN
                DELETE FROM entry_tags WHERE entry_id = OLD.entry_id;
            END;",
            (),
        )?;
        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS delete_unused_tags
            AFTER DELETE ON entry_tags
            BEGIN
                DELETE FROM tags WHERE tag_id NOT IN (SELECT tag_id FROM entry_tags);
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

    fn edit_entry(&mut self, entry: &mut Entry) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE entries SET entry_title = ?1, entry_content = ?2 WHERE entry_id = ?3",
            (&entry.title, &entry.content, &entry.id),
        )?;
        if let Some(tags) = entry.tags.clone() {
            self.conn.execute(
                "DELETE FROM entry_tags WHERE entry_id = ?1",
                (&entry.id,),
            )?;
            for mut tag in tags {
                if let Ok(tag_id) = self.create_tag(&tag.tag) {
                    tag.id = tag_id;
                    self.conn.execute(
                        "INSERT INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                        (&entry.id, &tag.id),
                    )?;
                }
            }
        }
        self.update_entries()?;
        Ok(())
    }

    // fn create_entry(&mut self, entry_title: &str, entry_content: &str, tags: Option<Vec<String>>) 
    fn create_entry(&mut self, entry: &mut Entry) // -> Result<(), rusqlite::Error> {
            -> Result<(), rusqlite::Error> {
        // let conn = Connection::open(&self.filename)?;
        self.conn.execute(
            "INSERT INTO entries (entry_title, entry_content)
            VALUES (?1, ?2)",
            (&entry.title, &entry.content),
        )?;
        entry.id = self.conn.last_insert_rowid() as u32;
        if let Some(tvec) = entry.tags.clone() {
            for mut tag in tvec {
                // self.conn.execute(
                //     "INSERT INTO tags (tag) values (?1)",
                //     (&tag,),
                // )?;
                // let tag_id = self.conn.last_insert_rowid();
                tag.id = self.create_tag(&tag.tag)?;
                self.conn.execute(
                    "INSERT INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                    (&entry.id, &tag.id)
                )?;
            }
        }
        self.update_entries()?;
        Ok(())
    }

    fn delete_entry(&mut self, entry: &Entry) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "DELETE FROM entries WHERE entry_id = ?1",
            (&entry.id,),
        )?;
        self.update_entries()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn prep_test(filename: &str) -> Db {
        match fs::remove_file(filename) {
            Ok(()) => println!("removed {}", filename),
            Err(e) => println!("{}", e),
        }
        let db = Db::new(filename);
        db.initialize_db().unwrap();
        db
    }

    #[test]
    fn basic_db_function() {
        let mut db = prep_test("test.db");
        let mut entry = Entry::new(
            "Test1".to_string(),
            "Test Content".to_string(),
            Some(vec![Tag::new("foo".to_string()), Tag::new("bar".to_string())]),
        );
        db.create_entry(&mut entry).unwrap();
        db.update_entries().unwrap();
        assert_eq!(db.entries[0].title, "Test1".to_string());
        assert_eq!(db.entries[0].content, "Test Content".to_string());
        assert_eq!(
            db.entries[0].tags.as_ref().unwrap().iter().map(|t| t.clone().tag).collect::<Vec<String>>(), 
            entry.tags.unwrap().iter().map(|t| t.clone().tag).collect::<Vec<String>>()
        );
        db.conn.close().unwrap();
        
    }

    #[test]
    fn test_insert_tag() {
        let mut db = prep_test("test2.db");
        let x = db.create_tag("foo").unwrap();
        assert_eq!(x, 1);
        let y = db.create_tag("bar").unwrap();
        assert_eq!(y, 2);
        let z = db.create_tag("foo").unwrap();
        assert_eq!(z, 1);
        db.conn.close().unwrap();
    }

    #[test]
    fn test_edit_entry() {
        let mut db = prep_test("test3.db");
        let mut entry = Entry::new(
            "Title!!".to_string(), 
            "content!!".to_string(), 
            Some(vec![Tag::new("Turkey".to_string()), Tag::new("Cheese".to_string())]));
        db.create_entry(&mut entry).unwrap();
        entry.title = "new title!!".to_string();
        let newtags = Some(vec![Tag::new("chicken".to_string()), Tag::new("salad".to_string())]);
        entry.tags = newtags;
        db.edit_entry(&mut entry).unwrap();
        assert_eq!(db.get_entries()[0].title, "new title!!".to_string());
        assert_eq!(db.get_entries()[0].tags.as_ref().unwrap()[0].tag, "chicken".to_string());
    }
    
    #[test]
    fn test_delete_entry() {
        let mut db = prep_test("test4.db");
        let mut entry = Entry::new(
            String::from("TITLE"),
            String::from("CONTENT"),
            Some(vec![Tag::new(String::from("DELETE")), Tag::new(String::from("ME"))])
        );
        db.create_entry(&mut entry).unwrap();
        assert_eq!(db.get_entries()[0].tags.as_ref().unwrap().len(), 2);
        db.delete_entry(&entry).unwrap();
        assert_eq!(db.get_entries().len(), 0);
    }
}