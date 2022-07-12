use glob::{glob_with, MatchOptions};
use std::{error::Error, fs, path::PathBuf};
use tui::widgets::ListState;

pub enum AppMode {
    Normal,
    Insert,
}

pub type ErrorBox = Box<dyn Error>;

pub struct App {
    pub list: StatefulList<PathEntry>,
    pub curret_input: String,
    pub app_mode: AppMode,
    pub pattern: String,
    glob_options: MatchOptions,
}

impl App {
    pub fn new() -> Self {
        App {
            list: StatefulList::new(),
            curret_input: String::new(),
            app_mode: AppMode::Normal,
            pattern: String::new(),
            glob_options: MatchOptions::new(),
        }
    }

    pub fn set_app_mode(&mut self, app_mode: AppMode) {
        self.app_mode = app_mode;
    }

    pub fn on_tick(&self) {
        // todo!("update me!")
    }

    pub fn set_pattern(&mut self, pattern: &str) -> Result<(), ErrorBox> {
        self.pattern = pattern.to_string();
        let entries = self.search_with_pattern()?;
        self.update_list(entries);
        Ok(())
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.glob_options.case_sensitive
    }

    pub fn toggle_case_sensitive(&mut self) {
        self.glob_options.case_sensitive = !self.glob_options.case_sensitive;
    }

    fn search_with_pattern(&self) -> Result<Vec<PathEntry>, ErrorBox> {
        let entries: Vec<PathEntry> = glob_with(&self.pattern.as_ref(), self.glob_options)?
            .filter_map(Result::ok)
            .map(PathEntry::new)
            .collect();
        Ok(entries)
    }

    fn update_list(&mut self, entries: Vec<PathEntry>) {
        self.list = StatefulList::with_items(entries);
    }

    pub fn toggle_delete(&mut self) {
        let i = self.list.get_index();
        if i.is_none() {
            return;
        }

        let i = i.unwrap();

        self.list.items[i].toggle_delete();
    }

    pub fn get_entries_by<P>(&self, mut predicate: P) -> Vec<PathEntry>
    where
        P: FnMut(&PathEntry) -> bool,
    {
        self.list
            .items
            .iter()
            .filter(|e| predicate(e))
            .cloned()
            .collect::<Vec<_>>()
    }

    pub fn delete_active_entries(&mut self) -> Result<(), ErrorBox> {
        let entries_to_delete = self.get_entries_by(|e| e.is_delete());
        for entry in entries_to_delete.iter() {
            if entry.is_file {
                fs::remove_file(entry.pathbuf.to_owned())?
            } else {
                fs::remove_dir_all(entry.pathbuf.to_owned())?
            }
        }

        let entries = self.get_entries_by(|e| !e.is_delete());
        self.update_list(entries);

        Ok(())
    }
}
#[derive(Clone)]
pub struct PathEntry {
    pub pathbuf: PathBuf,
    pub is_file: bool,
    _is_delete: bool,
}

impl PathEntry {
    pub fn new(pathbuf: PathBuf) -> Self {
        PathEntry {
            is_file: pathbuf.is_file(),
            pathbuf,
            _is_delete: true,
        }
    }

    pub fn toggle_delete(&mut self) {
        self._is_delete = !self._is_delete;
    }

    pub fn is_delete(&self) -> bool {
        self._is_delete
    }
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new() -> Self {
        StatefulList {
            state: ListState::default(),
            items: vec![],
        }
    }

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        let mut stateful_list = StatefulList {
            state: ListState::default(),
            items,
        };
        stateful_list.state.select(Some(0));
        stateful_list
    }

    pub fn get_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }

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

    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }

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

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
