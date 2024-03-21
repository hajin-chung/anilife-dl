package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"strings"
)

type CommandType string

var client *LifeClient

func main() {
	searchCmd := flag.NewFlagSet("search", flag.ExitOnError)
	downloadCmd := flag.NewFlagSet("download", flag.ExitOnError)
	infoCmd := flag.NewFlagSet("info", flag.ExitOnError)

	downloadEpisodes := downloadCmd.String("episodes", "", "")
	// downloadLimit := downloadCmd.Int("limit", 1, "")

	downloadLimit := 10
	client = NewLifeClient(downloadLimit * 1024)

	if len(os.Args) < 2 {
		printHelp()
		return
	}

	switch os.Args[1] {
	case "search":
		handleSearch(searchCmd)
	case "info":
		handleInfo(infoCmd)
	case "download":
		handleDownload(downloadCmd, downloadEpisodes)
	default:
		printHelp()
	}
}

func printHelp() {
	fmt.Println("anilfie-dl")
	fmt.Println("Usage:")
	fmt.Println("  anilife-dl search <query>")
	fmt.Println("  anilife-dl info <anime id>")
	fmt.Println("  anilife-dl download <anime id>")
	fmt.Println("  anilife-dl download --episodes <episode number>,... <anime id>")
}

func handleSearch(searchCmd *flag.FlagSet) {
	if err := searchCmd.Parse(os.Args[2:]); err != nil {
		fmt.Println("Failed to parse search command")
		os.Exit(1)
	}
	if searchCmd.NArg() != 1 {
		fmt.Println("You must provide a search query")
		os.Exit(1)
	}
	query := searchCmd.Arg(0)
	fmt.Printf("Searching for: %s\n", query)
	infos, err := client.Search(query)
	for _, info := range infos {
		fmt.Printf("%4s | %s\n", info.Id, info.Title)
	}
	if err != nil {
		log.Fatal(err)
	}
}

func handleInfo(infoCmd *flag.FlagSet) {
	if err := infoCmd.Parse(os.Args[2:]); err != nil {
		fmt.Println("Failed to parse info command")
		os.Exit(1)
	}
	if infoCmd.NArg() != 1 {
		fmt.Println("You must provide a anime id")
		os.Exit(1)
	}

	animeId := infoCmd.Arg(0)

	anime, err := client.GetAnime(animeId)
	if err != nil {
		fmt.Printf("error on GetAnime: %s", err)
		os.Exit(1)
	}

	fmt.Printf("# %s\n", anime.Info.Title)
	for _, episode := range anime.Episodes {
		fmt.Printf("%3s | %s\n", episode.Num, episode.Title)
	}
	if err != nil {
		log.Fatal(err)
	}
}

func handleDownload(downloadCmd *flag.FlagSet, targetEpisodes *string) {
	if err := downloadCmd.Parse(os.Args[2:]); err != nil {
		fmt.Println("Failed to parse download command")
		os.Exit(1)
	}
	if downloadCmd.NArg() != 1 {
		fmt.Println("You must provide an anime ID")
		os.Exit(1)
	}

	animeId := downloadCmd.Arg(0)

	fmt.Printf("Info on anime %s\n", animeId)
	anime, err := client.GetAnime(animeId)
	if err != nil {
		fmt.Printf("error on GetAnime: %s\n", err)
		os.Exit(1)
	}

	sanitizedTitle := strings.ReplaceAll(anime.Info.Title, "/", " ")
	os.MkdirAll(fmt.Sprintf("./%s", sanitizedTitle), 0755)
	fmt.Printf("Downloading %s\n", anime.Info.Title)
	if *targetEpisodes == "" {
		for _, episode := range anime.Episodes {
			fmt.Printf("Downloading %s %s\n", episode.Num, episode.Title)
			err = client.DownloadEpisode(anime, &episode)
			if err != nil {
				fmt.Printf("error on DownloadEpisode: %s", err)
				os.Exit(1)
			}
		}
	} else {
		eps := strings.Split(*targetEpisodes, ",")
		fmt.Printf("Downloading episodes: %v\n", eps)
		episodeIndexMap := map[string]bool{}

		for _, epi := range eps {
			episodeIndexMap[epi] = true
		}

		for _, episode := range anime.Episodes {
			if episodeIndexMap[episode.Num] == false {
				continue
			}

			fmt.Printf("Downloading %s %s\n", episode.Num, episode.Title)
			err = client.DownloadEpisode(anime, &episode)
			if err != nil {
				fmt.Printf("error on DownloadEpisode: %s", err)
				os.Exit(1)
			}
		}
	}
}
