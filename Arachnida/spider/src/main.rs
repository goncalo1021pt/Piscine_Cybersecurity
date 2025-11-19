use clap::Parser;
use std::path::PathBuf;

/// spider - downloads images from websites
#[derive(Parser)]
struct Args {
	/// The URL to scrape
	url: String,

	/// Recursive download images
	#[arg(short = 'r', long)]
	recursive: bool,

	/// Maximum recursion level
	#[arg(short = 'l', long, default_value = "5")]
	level: usize,

	#[arg(short = 'p', long, default_value = "./data/")]
	path: PathBuf,
}

fn fetch_html(url: &str) -> Result<String, Box<dyn std::error::Error>> {
	let client = reqwest::blocking::Client::builder()
		.user_agent("Mozilla/5.0 (Spider/1.0)")
		.build()?;
	
	let response = client.get(url).send()?;
	let body = response.text()?;
	Ok(body)
}

fn find_images(html: &str, base_url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
	use scraper::{Html, Selector};
	use url::Url;

	let document = Html::parse_document(html);
	let img_selector = Selector::parse("img").unwrap();

	let mut img_urls = Vec::new();

	for element in document.select(&img_selector) {
		// Try both src and srcset attributes
		let src = element.value().attr("src")
			.or_else(|| element.value().attr("data-src"));
			
		if let Some(src) = src {
			// Skip data URLs and empty strings
			if src.starts_with("data:") || src.is_empty() {
				continue;
			}
			
			match Url::parse(base_url)?.join(src) {
				Ok(absolute_url) => img_urls.push(absolute_url.to_string()),
				Err(e) => eprintln!("Failed to parse URL {}: {}", src, e),
			}
		}
	}
	Ok(img_urls)
}

fn is_valid_image(url: &str) -> bool {
	let lower_url = url.to_lowercase();
	
	// Must end with one of our extensions
	let valid_extensions = [".jpg", ".jpeg", ".png", ".gif", ".bmp"];
	let has_valid_ext = valid_extensions.iter().any(|ext| {
		// Check if URL ends with extension (before query params)
		lower_url.split('?').next().unwrap_or("").ends_with(ext)
	});
	
	// Skip SVG-based files and tiny thumbnails
	if lower_url.contains(".svg") || lower_url.contains("thumb") {
		return false;
	}
	
	has_valid_ext
}

fn download_image(url: &str, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
	use std::fs;
	use std::io::Write;

	fs::create_dir_all(path)?;

	let response = reqwest::blocking::get(url)?;
	let bytes = response.bytes()?;

	let filename = url.split('/').last().unwrap_or("image");
	let filepath = path.join(filename);

	let mut file = fs::File::create(&filepath)?;
	file.write_all(&bytes)?;
	Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();
	println!("Fetching: {}", args.url);

	let html = fetch_html(&args.url)?;
	println!("Downloaded {} bytes", html.len());
	
	let images = find_images(&html, &args.url)?;
	
	// Filter for valid image extensions
	let valid_images: Vec<String> = images.into_iter()
		.filter(|url| is_valid_image(url))
		.collect();
	
	println!("Found {} valid images:", valid_images.len());
	for img_url in &valid_images {
		println!("  {}", img_url);
		download_image(img_url, &args.path)?;
	}
	Ok(())
}
