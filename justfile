install-deps-docs:
  cargo install mdbook
  cargo install mdbook-plantuml

build_docs:
  mdbook build ./book/
