.PHONY: all

all: build-windows-32


# only used internally
setup-transpath:
	cd deps && pip install timm-0.5.4.tar

# only used internally
ismil-export:
	python -c "from pyfunctions.ismil_predict import convert; convert()" 

# only used internally
create-transpath-model:
	python create_transpath.py

create-windows-build: 
	cp deps/libtorch-2.7.0_w64/lib/*.dll build/  
	cp deps/openslide-win64/bin/*.dll build/  
	cp deps/vips-dev-8.15_w64/bin/*.dll build/
	cp install_packages.py build/
	mkdir -p build/models
	mkdir -p build/data 
	cp data/stats.json build/data/
	mkdir -p build/pyfunctions
	cp -r pyfunctions/*.py build/pyfunctions
	cp pyfunctions/requirements_pinned_windows.txt build/pyfunctions/; \
	cp pyfunctions/.requirements_pinned_windows.txt build/pyfunctions/; \
	cp target/release/rusty_slides.exe build/
	cp install_packages.py build/
	mkdir -p build/models 
	mv models/wsi.backbone.pth build/models/
	mv models/wsi.extractor.pth build/models/
	mv models/mri.pth build/models/


create-linux-build: 
	mkdir -p build/models
	mkdir -p build/data 
	mkdir -p build/deps
	mv deps/libtorch_linux_c11 build/deps/
	cp data/stats.json build/data/
	cp install_packages.py build/
	mkdir -p build/pyfunctions
	cp -r pyfunctions/*.py build/pyfunctions
	cp pyfunctions/requirements_pinned_linux.txt build/pyfunctions/; \
	cp pyfunctions/.requirements_pinned_linux.txt build/pyfunctions/; \
	cp target/release/rusty_slides* build/
	cp install_packages.py build/
	mkdir -p build/models 
	mv models/wsi.backbone.pth build/models/
	mv models/wsi.extractor.pth build/models/
	mv models/mri.pth build/models/

setup-tex:
	sudo pacman -S texlive-fontsrecommended \
			extra/texlive-latex \
			extra/texlive-latexextra \
			extra/texlive-plaingeneric

install-pydeps-linux:
	pip install -r pyfunctions/requirements.txt
	pip install -e deps/timm-0.5.4

build-linux:
	cp Cargo.toml.linux Cargo.toml
	LIBTORCH=${shell pwd}/deps/libtorch_linux_c11 cargo build --release
	#LIBTORCH_USE_PYTORCH=1 cargo build --release

test-linux:
	cp Cargo.toml.linux Cargo.toml
	mv models/wsi.backbone.pth models/wsi.backbone.pth.bak
	mv models/wsi.extractor.pth models/wsi.extractor.pth.bak
	cp models/mock.backbone.pth models/wsi.backbone.pth 
	cp models/mock.extractor.pth models/wsi.extractor.pth 
	cp tests/data/02a7b258e875cf073e2421d67ff824cd.tiff tests/data/mock.pred.tiff
	cp config.json config.json.bak
	cp tests/data/config.json config.json
	
	LIBTORCH=${shell pwd}/deps/libtorch_linux_c11 CACHE_MAX=512. \
	LD_LIBRARY_PATH=deps/libtorch_linux_c11/lib cargo test --release -- --test-threads=1;
	mv models/wsi.backbone.pth.bak models/wsi.backbone.pth; \
	mv models/wsi.extractor.pth.bak models/wsi.extractor.pth
	mv config.json.bak config.json

test-linux-debug:
	cp Cargo.toml.linux Cargo.toml
	mv models/wsi.backbone.pth models/wsi.backbone.pth.bak
	mv models/wsi.extractor.pth models/wsi.extractor.pth.bak
	cp models/mock.backbone.pth models/wsi.backbone.pth 
	cp models/mock.extractor.pth models/wsi.extractor.pth 
	cp tests/data/02a7b258e875cf073e2421d67ff824cd.tiff tests/data/mock.pred.tiff
	cp config.json config.json.bak
	cp tests/data/config.json config.json
	
	LIBTORCH=${shell pwd}/deps/libtorch_linux_c11 CACHE_MAX=512. \
	LD_LIBRARY_PATH=deps/libtorch_linux_c11/lib cargo test $(FILE) --release -- --test-threads=1
	LIBTORCH=${shell pwd}/deps/libtorch_linux_c11 CACHE_MAX=512. \
	LD_LIBRARY_PATH=deps/libtorch_linux_c11/lib cargo test $(FILE) --doc --release -- --test-threads=1;
	mv models/wsi.backbone.pth.bak models/wsi.backbone.pth; \
	mv models/wsi.extractor.pth.bak models/wsi.extractor.pth;
	mv config.json.bak config.json

build-windows-32:
	cp Cargo.toml.windows Cargo.toml
	cp pkgconfig/vips_32.pc pkgconfig/vips.pc
	PKG_CONFIG=i686-pc-linux-gnu-pkg-config \
	PKG_CONFIG_PATH=/home/mbu93/Projects/RustySlides/pkgconfig \
	LD_LIBRABRY_PATH=deps/openslide-win32/lib \
	PYO3_CROSS_PYTHON_VERSION=3.11 \
	LIBTORCH_USE_PYTORCH=1 \
	cargo build --release --target i686-pc-windows-gnu

build-windows-64:
	cp Cargo.toml.windows Cargo.toml
	cp pkgconfig/vips_64.pc pkgconfig/vips.pc
	PKG_CONFIG=x86_64-pc-linux-gnu-pkg-config \
	PKG_CONFIG_PATH=/home/mbu93/Projects/RustySlides/pkgconfig \
	LD_LIBRABRY_PATH=deps/openslide-win64/lib \
	PYO3_CROSS_PYTHON_VERSION=3.11 \
	LIBTORCH_USE_PYTORCH=1 \
	cargo build --release --target x86_64-pc-windows-gnu

build-windows-native: 
	# Build deps:
	# mingw
	# python3.11
	# pkgconfig-lite
	# llvm
	rm -r build/*.dll
	cp deps/libtorch-2.7.0_w64/lib/*.dll build/  
	cp deps/openslide-win64/bin/*.dll build/  
	cp deps/vips-dev-8.15_w64/bin/*.dll build/
	cp -r pyfunctions deps/
	cp Cargo.toml.windows Cargo.toml
	cp pkgconfig/vips_64.pc.windows pkgconfig/vips.pc
	cp pkgconfig/openslide.pc.windows pkgconfig/openslide.pc
	PWD_PATH=$$(pwd) && \
	PWD_DRIVE_LETTER=$$(echo $$PWD_PATH | cut -d '/' -f2) && \
	PWD_REST=$$(echo $$PWD_PATH | cut -d '/' -f3-) && \
	PWD_PATH_BACKSLASH=$$(echo "$$PWD_DRIVE_LETTER:\\\\\\\\$$PWD_REST" | sed 's/\//\\\\\\\\/g') && \
	echo "PWD=$$PWD_PATH_BACKSLASH" | cat - pkgconfig/openslide.pc.windows > temp && mv temp pkgconfig/openslide.pc && \
	echo "PWD=$$PWD_PATH_BACKSLASH" | cat - pkgconfig/vips_64.pc.windows > temp && mv temp pkgconfig/vips.pc
	LIBTORCH=${shell pwd}/deps/libtorch-2.7.0_w64 \
	TARGET=x86_64-pc-windows-msvc \
	PKG_CONFIG_PATH=$(shell pwd)/pkgconfig \
	cargo build --release 
	cp target/release/rusty_slides.exe build/

prepare-windows-test:
	mkdir -p target/release/deps
	rm -f target/release/deps/*.dll
	cp deps/libtorch-2.7.0_w64/lib/*.dll target/release/deps/  
	cp deps/openslide-win64/bin/*.dll target/release/deps/    
	cp deps/vips-dev-8.15_w64/bin/*.dll target/release/deps/
	cp tests/data/02a7b258e875cf073e2421d67ff824cd.tiff tests/data/mock.pred.tiff
	cp tests/data/config.json config.json	
	cp Cargo.toml.windows Cargo.toml
	cp pkgconfig/vips_64.pc.windows pkgconfig/vips.pc
	cp pkgconfig/openslide.pc.windows pkgconfig/openslide.pc
	mv models/wsi.backbone.pth models/wsi.backbone.pth.bak
	mv models/wsi.extractor.pth models/wsi.extractor.pth.bak
	cp models/mock.backbone.pth models/wsi.backbone.pth 
	cp models/mock.extractor.pth models/wsi.extractor.pth 
	PWD_PATH=$$(pwd) && \
	PWD_DRIVE_LETTER=$$(echo $$PWD_PATH | cut -d '/' -f2) && \
	PWD_REST=$$(echo $$PWD_PATH | cut -d '/' -f3-) && \
	PWD_PATH_BACKSLASH=$$(echo "$$PWD_DRIVE_LETTER:\\\\\\\\$$PWD_REST" | sed 's/\//\\\\\\\\/g') && \
	echo "PWD=$$PWD_PATH_BACKSLASH" | cat - pkgconfig/openslide.pc.windows > temp && mv temp pkgconfig/openslide.pc && \
	echo "PWD=$$PWD_PATH_BACKSLASH" | cat - pkgconfig/vips_64.pc.windows > temp && mv temp pkgconfig/vips.pc 
	rm -rf data/preprocessed

test-windows-native: prepare-windows-test
	LIBTORCH=${shell pwd}/deps/libtorch-2.7.0_w64 \
	TARGET=x86_64-pc-windows-msvc \
	PKG_CONFIG_PATH=$(shell pwd)/pkgconfig \
	LD_LIBRARY_PATH=deps/libtorch_linux_c11/lib \
	CACHE_MAX=512. \
	cargo test --release  -- --test-threads=1 
	mv models/wsi.backbone.pth.bak models/wsi.backbone.pth; \
	mv models/wsi.extractor.pth.bak models/wsi.extractor.pth
