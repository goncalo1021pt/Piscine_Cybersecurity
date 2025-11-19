.PHONY: all build run clean fclean re

# Build the project
all: build

build:
	cargo build --release

# Run the spider
run:
	cargo run --

# Clean downloaded images
clean:
	rm -rf ./data/
	cargo clean

# Full clean (images + build artifacts)
fclean: clean
	rm -rf target/

# Rebuild everything
re: fclean all
