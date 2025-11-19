all: build

build:
	cargo build --release

clean:
	rm -rf ./data/

fclean: clean
	cargo clean
	rm -rf target/

re: fclean all

.PHONY: all build run clean fclean re