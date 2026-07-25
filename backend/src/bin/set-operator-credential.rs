use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use rand::rngs::OsRng;

fn parse_args(args: &[String]) -> Result<(String, String), String> {
    let mut username: Option<String> = None;
    let mut output: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--username" => {
                username = args.get(i + 1).cloned();
                i += 2;
            }
            "--output" => {
                output = args.get(i + 1).cloned();
                i += 2;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let username = username.ok_or("missing required --username <name>")?;
    let output = output.ok_or("missing required --output <path>")?;
    Ok((username, output))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (username, output) = parse_args(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        eprintln!("usage: set-operator-credential --username <name> --output <path>");
        std::process::exit(1);
    });

    eprint!("Password: ");
    let mut password = String::new();
    std::io::stdin()
        .read_line(&mut password)
        .expect("failed to read password from stdin");
    let password = password.trim_end_matches(['\n', '\r']);

    if password.is_empty() {
        eprintln!("password must not be empty");
        std::process::exit(1);
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash_password failed")
        .to_string();

    let json = serde_json::json!({
        "username": username,
        "password_hash": password_hash,
    });

    std::fs::write(
        &output,
        serde_json::to_string_pretty(&json).expect("serialize failed"),
    )
    .unwrap_or_else(|e| {
        eprintln!("failed to write credential file {output}: {e}");
        std::process::exit(1);
    });

    println!("Operator credential written to {output}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_username_and_output() {
        let args = vec![
            "--username".to_string(),
            "operator".to_string(),
            "--output".to_string(),
            "/data/operator-credential.json".to_string(),
        ];
        let (username, output) = parse_args(&args).expect("parse failed");
        assert_eq!(username, "operator");
        assert_eq!(output, "/data/operator-credential.json");
    }

    #[test]
    fn should_reject_missing_username() {
        let args = vec!["--output".to_string(), "/data/cred.json".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn should_reject_missing_output() {
        let args = vec!["--username".to_string(), "operator".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn should_reject_unknown_argument() {
        let args = vec!["--bogus".to_string(), "value".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err());
    }
}
