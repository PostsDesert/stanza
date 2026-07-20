// Utility to generate password hashes for seeding

use stanza_backend::utils::hash_password;

fn main() {
    let password = "password123";
    
    match hash_password(password) {
        Ok((hash, salt)) => {
            println!("Password: {}", password);
            println!("Hash: {}", hash);
            println!("Salt: {}", salt);
            println!("\nFor seed.sh:");
            println!("HASH='{}'", hash);
            println!("SALT='{}'", salt);
        }
        Err(e) => {
            eprintln!("Error hashing password: {}", e);
            std::process::exit(1);
        }
    }
}
