package main

import (
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"slices"
	"strings"
)

func main() {
	slog.SetLogLoggerLevel(slog.LevelInfo)

	args := os.Args
	slog.Debug("main", "args", args)

	if len(args) < 2 {
		HandleHelp()
		return	
	}

	cmd := args[1]
	switch cmd {
	case "search":
		HandleSearch(args)
	case "list":
		HandleList(args)
	case "download":
		HandleDownload(args)
	default:
		HandleHelp()
	}
}

func HandleSearch(args []string) {
	if len(args) != 3 {
		slog.Error("HandleSearch", "error", "no search query provided")
		HandleHelp()
		return
	}

	client := NewAnilifeClient()
	query := args[2]
	animes, err := client.Search(query)
	if err != nil {
		slog.Error("HandleSearch", "error", err)
		HandleHelp()
		return
	}

	fmt.Printf("Search results on \"%s\"\n", query)
	for _, anime := range animes {
		fmt.Println(anime)
	}
}

func HandleList(args []string) {
	if len(args) != 3 {
		slog.Error("HandleList", "error", "no anime_id provided")
		HandleHelp()
		return
	}

	client := NewAnilifeClient()
	animeId := args[2]
	anime, episodes, err := client.GetAnime(animeId)
	if err != nil {
		slog.Error("HandleList", "error", err)
		HandleHelp()
		return
	}

	fmt.Println(anime)
	fmt.Println("========================================================")
	for _, episode := range episodes {
		fmt.Println(episode)
	}
}

func HandleDownload(args []string) {
	if len(args) != 4 {
		slog.Error("HandleList", "error", "no anime_id provided")
		HandleHelp()
		return
	}

	client := NewAnilifeClient()
	animeId := args[2]
	anime, episodes, err := client.GetAnime(animeId)
	if err != nil {
		slog.Error("HandleDownload", "error", err)
		HandleHelp()
		return
	}
	fmt.Println(anime)

	target := []*Episode{}
	if args[3] == "-a" || args[3] == "--all" {
		target = episodes
	} else {
		nums := strings.Split(args[3], ",")
		for _, e := range episodes {
			if slices.Contains(nums, e.Num) {
				target = append(target, e)
			}
		}
	}

	// make dir with anime title if it doesn't exist
	path := SanitizeFilename(anime.Title)
	err = os.MkdirAll(path, 0755)
	if err != nil {
		slog.Error("HandleDownload", "error", err)
		HandleHelp()
		return
	}

	for _, e := range target {
		fmt.Println(e)
		url, err := client.GetEpisodeHLS(e, anime)
		if err != nil {
			slog.Error("HandleDownload", "error", err)
			HandleHelp()
			return
		}
		filename := fmt.Sprintf("%-2s %s.mp4", e.Num, e.Title)
		filename = SanitizeFilename(filename)
		out := filepath.Join(path, filename)

		client.Download(url, out)
	}
}

func HandleHelp() {
	fmt.Println("anime-dl")
	fmt.Println("Usage:")
	fmt.Println("  anime-dl search <query>")
	fmt.Println("  anime-dl list <anime_id>")
	fmt.Println("  anime-dl download <anime_id> <episode_num1>,<episode_num2>,...")
	fmt.Println("  anime-dl download <anime_id> -a")
	fmt.Println("Options:")
	fmt.Println("  -h --help      Show this screen")
	fmt.Println("  -a --all          Download all episodes")
}

