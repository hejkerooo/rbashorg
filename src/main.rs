extern crate directories;

use std::{fs, io};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use directories::{ProjectDirs};
use std::io::Write;
use rand::seq::SliceRandom;

struct DirManager {
    base_dir: String,
}

impl DirManager {
    pub fn prepare() -> DirManager {
        let base_dirs = ProjectDirs::from("com", "RBashOrg", "RBashOrg").expect("failed to load base local data");

        let dir = base_dirs.data_local_dir().to_str().unwrap();

        fs::create_dir_all(&dir).unwrap();

        DirManager {
            base_dir: dir.to_owned(),
        }
    }

    pub fn indexes_exists(&self) -> bool {
        self.get_indexes_path().exists()
    }

    pub fn get_indexes_path(&self) -> PathBuf {
        self.construct_file_path("indexes.txt")
    }


    pub fn content_exists(&self) -> bool {
        self.get_content_path().exists()
    }

    pub fn get_content_path(&self) -> PathBuf {
        self.construct_file_path("bashorg.txt")
    }

    fn construct_file_path(&self, file_name: &str) -> PathBuf {
        return [
            &self.base_dir,
            &PathBuf::from(file_name).to_str().unwrap().to_string(),
        ].iter().collect();
    }
}

fn read_bytes<P>(filename: P) -> io::Result<io::Bytes<io::BufReader<File>>> where P: AsRef<Path>, {
    let file = File::open(filename)?;

    Ok(io::BufReader::new(file).bytes())
}

fn download_file(dest: &PathBuf) -> io::Result<String> {
    let response = reqwest::blocking::get("http://bash.org.pl/text")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if !response.status().is_success() {
        return Err(io::Error::new(io::ErrorKind::Other, "Request failed"));
    }

    let mut file = File::create(dest)?;

    let content = response.bytes().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut content = Cursor::new(content);
    io::copy(&mut content, &mut file)?;

    Ok(dest.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Path contains invalid UTF-8"))?.to_owned())
}

fn get_byte_pos(file_path: &PathBuf) -> io::Result<Vec<(u32, u32)>> {
    let mut pos: Vec<(u32, u32)> = vec![];
    let mut current_pos: u32 = 0;

    let bytes = read_bytes(file_path)?;

    for line in bytes {
        current_pos += 1;

        if line.is_err() {
            continue;
        }

        if line.unwrap() == b'%' {
            match pos.last() {
                None => pos.push((0, current_pos)),
                Some(&(_start, end)) => {
                    if current_pos - end == 0 {
                        continue;
                    }

                    pos.push((end.clone(), current_pos))
                }
            }
        }
    }

    Ok(pos)
}

fn get_random(indexes: &Vec<(u32, u32)>) -> Option<&(u32, u32)> {
    indexes.choose(&mut rand::thread_rng())
}

fn print_chunks(joke_chunk: Utf8Chunk, append_bytes: &mut Vec<u8>) {
    for j in joke_chunk {
        let (mut bytes, _start, _end) = j;

        if let Some(last_byte) = bytes.last() {
            if *last_byte == b'%' {
                bytes.pop();
            }
        }

        match std::str::from_utf8(&bytes) {
            Ok(content_str) => {
                write!(io::stdout(), "{}", content_str).expect("Cannot write to stdout");
                append_bytes.clear();
            }
            Err(e) => {
                if e.valid_up_to() < bytes.len() {
                    append_bytes.extend_from_slice(&bytes[e.valid_up_to()..])
                }
            }
        }
    }
}

fn prepare_chunks(dir_manager: DirManager, destination: &PathBuf) -> Utf8Chunk {
    let pos = if !dir_manager.indexes_exists() { Some(get_byte_pos(&destination)) } else { None }.expect("Failed to read byte pos");

    let binding = pos.unwrap();

    let (start, end) = get_random(&binding).unwrap();

    let interval: u32 = 5;

    let joke_chunk = Utf8Chunk {
        index: 0,
        chunks: divide_range_into_intervals(*start, *end, interval)
            .iter().map(|(s, e)| {
            Joke {
                start: *s,
                end: *e,
            }
        }).collect(),
        path: dir_manager.get_content_path(),
    };
    joke_chunk
}

struct Joke {
    pub start: u32,
    pub end: u32,
}

struct Utf8Chunk {
    pub chunks: Vec<Joke>,
    pub index: usize,
    pub path: PathBuf,
}

impl Iterator for Utf8Chunk {
    type Item = (Vec<u8>, u32, u32);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(j) = self.chunks.get(self.index) {
            self.index += 1;

            Some(get_partial_bytes(&self.path, &j.start, &j.end))
        } else {
            None
        }
    }
}

fn divide_range_into_intervals(start: u32, end: u32, interval_length: u32) -> Vec<(u32, u32)> {
    let mut intervals = Vec::new();

    let mut current_start = start;
    while current_start < end {
        let current_end = std::cmp::min(current_start + interval_length, end);
        intervals.push((current_start, current_end));
        current_start = current_end;
    }

    intervals
}
fn get_partial_bytes(destination: &PathBuf, start: &u32, end: &u32) -> (Vec<u8>, u32, u32) {
    let mut content_handle: File = File::open(&destination).expect("Failed to open file");

    content_handle.seek(SeekFrom::Start(*start as u64)).expect("Failed to seek byte range");

    let mut buf = vec![0; (*end - *start) as usize];

    content_handle.read_exact(&mut buf).unwrap();

    (buf, *start, *end)
}

fn main() {
    let dir_manager = DirManager::prepare();

    let destination = dir_manager.get_content_path();

    if !dir_manager.content_exists() {
        download_file(&destination).expect("Failed to download bash org file");
    }

    let joke_chunk = prepare_chunks(dir_manager, &destination);

    let mut append_bytes: Vec<u8> = vec![];

    print_chunks(joke_chunk, &mut append_bytes);
}
