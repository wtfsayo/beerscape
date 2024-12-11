use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};
use rand::Rng;
use reqwest::Client;
use std::collections::HashSet;
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

const TOTAL_RECIPES_TARGET: usize = 10_000;
const MIN_RECIPE_ID: u32 = 1;
const MAX_RECIPE_ID: u32 = 4_000_000;
const CONCURRENT_REQUESTS: usize = 10;

#[derive(Debug)]
struct DownloadStats {
    successful: usize,
    failed: usize,sd
    total_attempted: usize,
    existing: usize,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create recipes directory if it doesn't exist
    fs::create_dir_all("recipes")?;

    // Scan existing recipes
    let mut existing_recipes = HashSet::new();
    println!("Scanning existing recipes...");
    for entry in glob("recipes/*.bsmx")? {
        if let Ok(path) = entry {
            if let Some(file_stem) = path.file_stem() {
                // Store the full filename to track duplicates
                if let Some(name) = file_stem.to_str() {
                    existing_recipes.insert(name.to_string());
                }
            }
        }
    }

    println!("Found {} existing recipes", existing_recipes.len());
    let remaining_needed = TOTAL_RECIPES_TARGET.saturating_sub(existing_recipes.len());
    println!("Need to download {} more recipes", remaining_needed);

    if remaining_needed == 0 {
        println!("Target already reached! No more downloads needed.");
        return Ok(());
    }

    // Create a new HTTP client with timeout
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    let mut stats = DownloadStats {
        successful: existing_recipes.len(),
        failed: 0,
        total_attempted: 0,
        existing: existing_recipes.len(),
    };

    // Setup progress bar
    let pb = ProgressBar::new(TOTAL_RECIPES_TARGET as u64);
    pb.set_position(existing_recipes.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:50.cyan/blue}] {pos}/{len} ({percent}%) - ETA: {eta_precise} - Success: {msg}")?
        .progress_chars("#>-"));
    pb.set_message(format!(
        "{}/{} (Failed: {})",
        stats.successful, stats.total_attempted, stats.failed
    ));

    let mut rng = rand::thread_rng();
    let mut attempted_ids = HashSet::new();

    while stats.successful < TOTAL_RECIPES_TARGET {
        let mut current_batch = vec![];

        // Generate batch of new IDs
        while current_batch.len() < CONCURRENT_REQUESTS {
            let id = rng.gen_range(MIN_RECIPE_ID..=MAX_RECIPE_ID);
            if !attempted_ids.contains(&id) {
                current_batch.push(id);
                attempted_ids.insert(id);
            }
        }

        let mut tasks = vec![];

        for id in current_batch {
            let client = client.clone();
            let pb = pb.clone();

            tasks.push(tokio::spawn(async move {
                match download_recipe(&client, id).await {
                    Ok(Some(info)) => (id, true, Some(info)),
                    Ok(None) => (id, false, None),
                    Err(e) => {
                        eprintln!("Error downloading recipe {}: {}", id, e);
                        (id, false, None)
                    }
                }
            }));
        }

        // Wait for all tasks in batch to complete
        for task in tasks {
            match task.await {
                Ok((id, success, info)) => {
                    if success && info.is_some() {
                        stats.successful += 1;
                        pb.set_position(stats.successful as u64);
                    } else {
                        stats.failed += 1;
                        attempted_ids.remove(&id);
                    }
                    stats.total_attempted += 1;
                    pb.set_message(format!(
                        "{}/{} (Failed: {})",
                        stats.successful, stats.total_attempted, stats.failed
                    ));
                }
                Err(e) => {
                    eprintln!("Task error: {}", e);
                    stats.failed += 1;
                }
            }
        }

        // Small delay between chunks to avoid overwhelming the server
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    pb.finish_with_message(format!(
        "Completed: {}/{} successful",
        stats.successful, TOTAL_RECIPES_TARGET
    ));

    println!("\nDownload Summary:");
    println!("----------------");
    println!("Previously Existing: {}", stats.existing);
    println!("Newly Downloaded: {}", stats.successful - stats.existing);
    println!("Failed Attempts: {}", stats.failed);
    println!("Total Attempts: {}", stats.total_attempted);
    println!(
        "Final Success Rate: {:.1}%",
        ((stats.successful - stats.existing) as f64 / stats.total_attempted as f64) * 100.0
    );

    Ok(())
}

async fn download_recipe(
    client: &Client,
    recipe_id: u32,
) -> Result<Option<RecipeInfo>, Box<dyn Error>> {
    // Direct download URL
    let url = format!("https://redacted-recipes.com/download.php?id={}", recipe_id);

    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148")
        .send()
        .await?;

    if response.status().is_success() {
        // Get the filename from Content-Disposition header or use default
        let filename = response
            .headers()
            .get("content-disposition")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| {
                s.split("filename=")
                    .nth(1)
                    .map(|f| f.trim_matches('"').to_string())
            })
            .unwrap_or_else(|| format!("{}.bsmx", recipe_id));

        let content = response.bytes().await?;

        // Check if content seems valid (contains XML or BSMX data)
        if content.starts_with(b"<") {
            let file_path = Path::new("recipes").join(&filename);
            let mut file = File::create(file_path)?;
            file.write_all(&content)?;

            Ok(Some(RecipeInfo {
                id: recipe_id,
                filename,
            }))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
