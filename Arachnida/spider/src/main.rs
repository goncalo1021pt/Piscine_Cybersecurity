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
		let src = element.value().attr("src")
			.or_else(|| element.value().attr("data-src"));
			
		if let Some(src) = src {
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
	
	let valid_extensions = [".jpg", ".jpeg", ".png", ".gif", ".bmp"];
	let has_valid_ext = valid_extensions.iter().any(|ext| {
		lower_url.split('?').next().unwrap_or("").ends_with(ext)
	});
	
	if lower_url.contains(".svg") {
		return false;
	}
	
	has_valid_ext
}

fn download_image(url: &str, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
	use std::fs;
	use std::io::Write;

	fs::create_dir_all(path)?;

	let client = reqwest::blocking::Client::builder()
		.user_agent("Mozilla/5.0 (Spider/1.0)")
		.build()?;
	
	let response = client.get(url).send()?;
	let bytes = response.bytes()?;

	let mut filename = url.split('/').last().unwrap_or("image").to_string();
	
	if let Some(pos) = filename.find('?') {
		filename.truncate(pos);
	}
	
	filename = urlencoding::decode(&filename)?.into_owned();
	
	const MAX_LEN: usize = 200;
	if filename.len() > MAX_LEN {
		if let Some(ext_pos) = filename.rfind('.') {
			let ext = &filename[ext_pos..];
			let name_part = &filename[..MAX_LEN.saturating_sub(ext.len())];
			filename = format!("{}{}", name_part, ext);
		} else {
			filename.truncate(MAX_LEN);
		}
	}
	
	let filepath = path.join(&filename);

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
