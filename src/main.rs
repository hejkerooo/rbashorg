extern crate directories;

use std::{fs, io};
use std::fs::File;
use std::io::{Read, Seek, Write, SeekFrom};
use std::path::{PathBuf};
use directories::{ProjectDirs};
use rand::Rng;
use std::net::TcpStream;

struct DirManager {
    base_dir: String,
}

const HOST: &str = "bash.org.pl";

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

fn download_file(dest: &PathBuf) -> io::Result<()> {
    let mut stream = TcpStream::connect(format!("{}:80", HOST))?;

    let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", "/text", HOST);
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
        let mut file = File::create(dest)?;
        file.write_all(&response[index + 4..])?;
    } else {
        return Err(io::Error::new(io::ErrorKind::Other, "Response header not found"));
    }

    Ok(())
}

fn find_random_percent_positions(file_path: &PathBuf) -> std::io::Result<(u32, u32)> {
    let file = File::open(file_path).expect("Failed to load file");
    let file_size = file.metadata().expect("Failed to load file metadata").len() as u32;

    let mut rng = rand::thread_rng();
    let random_start_pos = rng.gen_range(0..file_size);

    let mut start_pos = random_start_pos;
    let mut end_pos = file_size;

    let mut next_percent_pos = None;

    while start_pos < end_pos {
        let (bytes, _, _) = get_partial_bytes(file_path, &start_pos, &end_pos);

        if let Some(pos) = bytes.iter().position(|&b| b == b'%') {
            next_percent_pos = Some(start_pos + pos as u32);
            break;
        }

        start_pos += bytes.len() as u32;
    }

    let mut prev_percent_pos = None;
    if let Some(pos) = next_percent_pos {
        start_pos = if pos > 1 { 0 } else { pos - 1 };
        end_pos = pos;

        while start_pos < end_pos {
            let (bytes, start, _) = get_partial_bytes(file_path, &start_pos, &end_pos);

            if let Some(pos) = bytes.iter().rposition(|&b| b == b'%') {
                prev_percent_pos = Some(start + pos as u32);
                break;
            }

            if start_pos == 0 {
                break;
            }

            start_pos = start.saturating_sub(bytes.len() as u32);
        }
    }

    match (prev_percent_pos, next_percent_pos) {
        (Some(prev), Some(next)) => Ok((prev + 1, next)),
        _ => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Percent signs not found")),
    }
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
    let pos = if !dir_manager.indexes_exists() { Some(find_random_percent_positions(&destination).expect("Failed to find percentage")) } else { None }.expect("Failed to read byte pos");

    let (start, end) = pos;

    let interval: u32 = 5;

    let joke_chunk = Utf8Chunk {
        index: 0,
        chunks: divide_range_into_intervals(start, end, interval)
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
