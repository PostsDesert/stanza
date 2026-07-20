use stanza_backend::{db, models::User, utils::hash_password};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if available
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:stanza.db".to_string());
    
    // Connect to DB
    let pool = db::init_pool(&database_url).await?;

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "list" => {
            let users = db::list_users(&pool).await?;
            if users.is_empty() {
                println!("No users found.");
            } else {
                println!("{:<36} | {:<30} | {:<20}", "ID", "Email", "Username");
                println!("{}", "-".repeat(90));
                for user in users {
                    println!("{:<36} | {:<30} | {:<20}", user.id, user.email, user.username);
                }
            }
        }
        "add" => {
            if args.len() != 5 {
                println!("Usage: manage_users add <email> <username> <password>");
                return Ok(());
            }
            let email = &args[2];
            let username = &args[3];
            let password = &args[4];

            println!("Adding user: {}", email);
            
            let (hash, salt) = hash_password(password).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            
            let user = User::new(email.clone(), username.clone(), hash, salt);

            match db::create_user(&pool, &user).await {
                Ok(_) => println!("User added successfully."),
                Err(e) => println!("Error adding user: {}", e),
            }
        }
        "remove" => {
            if args.len() != 3 {
                println!("Usage: manage_users remove <email>");
                return Ok(());
            }
            let email = &args[2];
            println!("Removing user: {}", email);
            match db::delete_user_by_email(&pool, email).await {
                Ok(_) => println!("User removed successfully."),
                Err(e) => println!("Error removing user: {}", e),
            }
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Usage: manage_users <command> [args]");
    println!("Commands:");
    println!("  list                            List all users");
    println!("  add <email> <username> <password> Add a new user");
    println!("  remove <email>                  Remove a user by email");
}
