use dirs::{config_local_dir, home_dir};
use std::ffi::OsString;
use std::fs::DirBuilder;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::input_n_state::AppParams;

const SHOOTER_DIR: &str = "shooter";
const SHOOTER_CONFIG: &str = "config.toml";

pub fn do_read_config() -> Result<AppParams, String> {
    match get_config_file_path() {
        Ok(path) => match read_config(&path) {
            Ok(param) => return Ok(param),
            Err(e) => return Err(format!("Failed to read config file. {}", e)),
        },
        Err(e) => return Err(format!("Failed to read config file. {}", e)),
    };
}

pub fn do_write_config(app_params: &AppParams) -> Result<(), String> {
    match get_config_file_path() {
        Ok(path) => match write_config(&path, &app_params) {
            Ok(_) => (),
            Err(e) => return Err(format!("Failed to write config file. {}", e)),
        },
        Err(e) => return Err(format!("Failed to write config file. {}", e)),
    };
    Ok(())
}

fn read_config(config_file_path_str: &OsString) -> Result<AppParams, String> {

    let config: File = match get_file(config_file_path_str, true) {
        Ok(file) => file,
        Err(e) => return Err(e),
    };

    let mut buf_reader = BufReader::new(config);
    let mut contents = String::new();
    let result = buf_reader.read_to_string(&mut contents);
    match result {
      Ok(_) => (),
      Err(e) => return Err(format!("Could not read file. {}",e)),
    };
    println!("file contents: {}", contents);

    //let app_param: AppParams  = toml::from_str(&contents).unwrap(); 
    match toml::from_str(&contents) {
      Ok(params) => Ok(params),
      Err(e) => return Err(format!("Could not construct AppParams from file. {}", e)),
    }
}

fn write_config(config_file_path_str: &OsString, app_params: &AppParams) -> Result<(), String> {
    let config: File = match get_file(config_file_path_str, false) {
        //let config:File  = match get_file(config_file_buf) {
        Ok(file) => file,
        Err(e) => return Err(e),
    };

    let toml = toml::to_string(app_params).unwrap();
    let mut writer = BufWriter::new(config);
    match writer.write(&toml.into_bytes()) {
        Err(e) => return Err(format!("Error writing. {}", e)),
        Ok(size) => println!("Wrote {} bytes to config.", size),
    }
    match writer.flush() {
        Ok(_) => (),
        Err(e) => {
            return Err(format!(
                "Could not open file the config file for writing {e}"
            ));
        }
    }
    Ok(())
}

pub fn get_config_file_path() -> Result<OsString, String> {
    let config_dir = match config_local_dir() {
        Some(config) => config,
        None => match home_dir() {
            Some(home) => home,
            None => return Err("No config dir for this system?".to_string()),
        },
    };
    let file_path: PathBuf = [SHOOTER_DIR, SHOOTER_CONFIG].iter().collect();
    let config_file_buf: PathBuf = config_dir.join(file_path);
    Ok(config_file_buf.into_os_string())
}

pub fn get_file(config_file_buf: &OsString, read_file: bool) -> Result<File, String> {
    let config_file: &Path = Path::new(&config_file_buf);
    //let config_file: &Path = config_file_buf.as_path();

    //Check dir containing the file exists, create if required.
    match config_file.parent() {
        Some(dir) => match dir.try_exists() {
            Ok(true) => {}
            Ok(false) => match DirBuilder::new().recursive(true).create(dir) {
                Ok(_) => {}
                Err(e) => {
                    return Err(format!(
                        "Failure to create directory:{}{}",
                        e,
                        dir.to_str().unwrap()
                    ))
                }
            },
            _ => return Err(format!("Error accessing directory: {}", SHOOTER_DIR)),
        },
        None => {
            return Err(format!(
                "Error accessing parent directory: {}",
                config_file.to_str().unwrap()
            ))
        }
    }

    //Open the file.
    match config_file.try_exists() {
        Ok(true) => match std::fs::OpenOptions::new().read(read_file).write(!read_file).open(config_file) {
            Ok(open_file) => Ok(open_file),
            Err(e) => {
                return Err(format!(
                    "Failure to open file:{} {}",
                    e,
                    config_file_buf.to_str().unwrap()
                ))
            }
        },
        Ok(false) => match File::create(config_file) {
            Ok(create_file) => Ok(create_file),
            Err(e) => {
                return Err(format!(
                    "Failure to create file:{} {}",
                    e,
                    config_file_buf.to_str().unwrap()
                ))
            }
        },
        _ => {
            let error = format!(
                "Error getting access to config file: {}",
                config_file_buf.to_str().unwrap()
            );
            return Err(error);
        }
    }
}

#[test]
fn test_get_file() {
    let path_string = get_config_file_path();
    let string_path = path_string.unwrap().into_string().unwrap();
    println!("Location of config file for shooter: {}", string_path);

    let tmp_dir_buf = std::env::temp_dir().join("shooter_config_test.toml");
    let tmp_dir_str = tmp_dir_buf.into_os_string();
    println!(
        "Writing test file to: {}",
        tmp_dir_str.clone().into_string().unwrap()
    );

    let app_params = AppParams {
        player_name: "shrubbo".to_string(),
    };
    match write_config(&tmp_dir_str, &app_params) {
        Ok(_) => {}
        Err(e) => assert!(false, "Failure to write config  {e}"),
    }
    let params = match read_config(&tmp_dir_str) {
        Ok(param) => param,
        Err(e) => {
          return assert!(false, "Failure to read config {e}")
        }, };

    assert!(params.player_name == app_params.player_name);
}
