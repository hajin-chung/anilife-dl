build:
	go build --ldflags '-linkmode external -extldflags "-static"' -o anilife-dl
run:
	go run .
