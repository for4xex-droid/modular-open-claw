.PHONY: build clean

build:
	@echo "🛠️ Compiling WASM Skill..."
	wasm-pack build --target web --release
	@echo "✅ Build completed! The output artifacts are in pkg/"

clean:
	rm -rf target pkg
