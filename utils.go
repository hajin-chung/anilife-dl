package main

import (
	"path/filepath"
	"regexp"
	"strings"
)

func SanitizeFilename(filename string) string {
	// Remove path separators to prevent path traversal
	filename = filepath.Base(filename)

	// Replace invalid characters with a safe alternative (e.g., underscore)
	// This regex targets common invalid characters for filenames on various OS
	re := regexp.MustCompile(`[<>:"/\\|?*\x00-\x1F]`) // Includes control characters
	filename = re.ReplaceAllString(filename, "_")

	// Optionally, trim leading/trailing spaces or periods (especially for Windows)
	filename = strings.TrimSpace(filename)
	filename = strings.TrimRight(filename, ".")

	return filename
}

