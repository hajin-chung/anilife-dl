.PHONY: build install

build:
	go build -ldflags "-linkmode 'external' -extldflags '-static'" -tags netgo,osusergo -o anilife-dl .

install: build
	cp anilife-dl /mnt/d/anime
