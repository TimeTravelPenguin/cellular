install-deps-docs:
  cargo install mdbook

build_docs:
  mdbook build ./book/
