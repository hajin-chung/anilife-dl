package main

import (
	"context"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"os"
	"os/exec"
	"runtime"
	"strings"

	"github.com/schollz/progressbar/v3"
	"golang.org/x/sync/semaphore"
)

func (a *AnilifeClient) Download(url string, path string) error {
	if err := os.MkdirAll(".tmp", 0755); err != nil {
		slog.Error("Download", "error", err)
		return err
	}
	defer os.RemoveAll(".tmp")

	req, err := http.NewRequest("GET", url, nil)
	if  err != nil {
		slog.Error("Download", "error", err)
		return err
	}
	req.Header.Add("Referer", ANILIFE_URL)
	res, err := a.client.Do(req)
	if  err != nil {
		slog.Error("Download", "error", err)
		return err
	}
	hlsBytes, err := io.ReadAll(res.Body)
	if  err != nil {
		slog.Error("Download", "error", err)
		return err
	}

	hlsContent := string(hlsBytes[:])
	segmentUrls := parseHls(hlsContent)
	slog.Debug("Download", "segmentUrls", segmentUrls)

	bar := progressbar.Default(int64(len(segmentUrls)))
	maxWorkers := runtime.GOMAXPROCS(0)
	sem := semaphore.NewWeighted(int64(maxWorkers))

	for i, url := range segmentUrls {
		if err := sem.Acquire(context.Background(), 1); err != nil {
			slog.Error("Download", "error", err)
			break
		}

		go func() {
			a.downloadSegment(sem, url, i)
			bar.Add(1)
		}()
	}

	if err := sem.Acquire(context.Background(), int64(maxWorkers)); err != nil {
		slog.Error("Download", "error", err)
		return err
	}

	// combine segments
	combinePath := ".tmp/all.ts"
	err = os.Remove(combinePath)
	if err != nil && !errors.Is(err, os.ErrNotExist) {
		slog.Error("Download", "error", err)
		return err
	}

	file, err := os.OpenFile(combinePath, os.O_APPEND|os.O_WRONLY|os.O_CREATE, 0644)
	if err != nil {
		slog.Error("Download", "error", err)
		return err
	}
	defer file.Close()

	bar = progressbar.Default(int64(len(segmentUrls)))

	for i := range len(segmentUrls) {
		segmentPath := fmt.Sprintf(".tmp/%d.seg", i)
		segmentFile, err := os.Open(segmentPath)
		if err != nil {
			slog.Error("Download", "error", err)
			return err
		}
		defer segmentFile.Close()

		bytesWritten, err := io.Copy(file, segmentFile)
		if err != nil {
			slog.Error("Download", "error", err)
			return err
		}

		bar.Add(1)
		slog.Debug("Download combine", "idx", i, "bytesWritten", bytesWritten)
	}
	
	cmd := exec.Command("ffmpeg", "-i", combinePath, "-c", "copy", path)
	if err := cmd.Run(); err != nil {
		slog.Error("Download", "error", err)
		return err
	}
	return nil
}

func (a *AnilifeClient) downloadSegment(sem *semaphore.Weighted, url string, idx int) {
	defer sem.Release(1)
	req, err := http.NewRequest("GET", url, nil)
	if  err != nil {
		slog.Error("downloadSegment", "error", err)
		return
	}
	req.Header.Add("Referer", ANILIFE_URL)
	req.Header.Add("Origin", ANILIFE_URL)

	res, err := a.client.Do(req)
	if  err != nil {
		slog.Error("downloadSegment", "error", err)
		return
	}

	file, err := os.Create(fmt.Sprintf(".tmp/%d.seg", idx))
	if err != nil {
		slog.Error("downloadSegment", "error", err)
		return
	}
	defer file.Close()

	bytesWritten, err := io.Copy(file, res.Body)
	if err != nil {
		slog.Error("downloadSegment", "error", err)
		return
	}
	slog.Debug("downloadSegment", "idx", idx, "bytesWritten", bytesWritten)
}

func parseHls(content string) []string {
	urls := []string{}
	lines := []string{}
	for line := range strings.Lines(content) {
		lines = append(lines, line)
	}

	for i := 0; i < len(lines); i++ {
		line := lines[i]
		if strings.HasPrefix(line, "#EXTINF") {
			url := lines[i+1]
			url = strings.TrimSpace(url)
			urls = append(urls, url)
			i++
		}
	}

	return urls
}
