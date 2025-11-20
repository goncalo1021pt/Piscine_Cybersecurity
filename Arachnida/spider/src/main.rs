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
	#[arg(short = 'l', long, default_value = "5", requires = "recursive")]
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

fn find_links(html: &str, base_url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
	use scraper::{Html, Selector};
	use url::Url;

	let document = Html::parse_document(html);
	let link_selector = Selector::parse("a").unwrap();
	let base = Url::parse(base_url)?;

	let mut links = Vec::new();

	for element in document.select(&link_selector) {
		if let Some(href) = element.value().attr("href") {
			if let Ok(absolute_url) = base.join(href) {
				if absolute_url.domain() == base.domain() {
					links.push(absolute_url.to_string());
				}
			}
		}
	}
	Ok(links)
}

fn crawl_recursive(url: &str,
	depth: usize,
	max_depth: usize,
	visited: &mut std::collections::HashSet<String>,
	save_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
	
	if depth > max_depth || visited.contains(url) {
		return Ok(());
	}
	
	visited.insert(url.to_string());
	println!("\n[Depth {}] Crawling: {}", depth, url);
	
	let html = fetch_html(url)?;
	let images = find_images(&html, url)?;
	let valid_images: Vec<String> = images.into_iter()
		.filter(|u| is_valid_image(u))
		.collect();
	
	println!("Found {} images", valid_images.len());
	for img_url in &valid_images {
		if let Err(e) = download_image(img_url, save_path) {
			eprintln!("Failed to download {}: {}", img_url, e);
		}
	}
	
	if depth < max_depth {
		let links = find_links(&html, url)?;
		for link in links {
			crawl_recursive(&link, depth + 1, max_depth, visited, save_path)?;
		}
	}
	
	Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	use std::collections::HashSet;
	
	let args = Args::parse();
	
	if args.recursive {
		println!("Starting recursive crawl (max depth: {})", args.level);
		let mut visited = HashSet::new();
		crawl_recursive(&args.url, 0, args.level, &mut visited, &args.path)?;
		println!("\nCrawl complete! Visited {} pages", visited.len());
	} else {
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
	}

	Ok(())
}
