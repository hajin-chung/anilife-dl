package main

import (
	"fmt"
	"log/slog"
	"testing"
)

func TestClient(t *testing.T) {
	slog.Info("TestGetAnime")

	c := NewAnilifeClient()
	anime, episodes, err := c.GetAnime("527")
	if err != nil {
		slog.Error("TestGetAnime", "error", err)
	}
	fmt.Printf("anime: %+v\n", anime)
	fmt.Printf("episodes\n")
	for _, episode := range episodes {
		fmt.Printf("%+v\n", episode)
	}

	episode := episodes[0]
	hlsUrl, err := c.GetEpisodeHLS(episode, anime)
	if err != nil {
		slog.Error("TestGetAnime", "error", err)
	}
	slog.Info("TestClient", "hls url", hlsUrl)
}
