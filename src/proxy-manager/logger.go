package main

import (
	"fmt"
	"strings"

	"github.com/docker/docker/api/types/container"
)

// ANSI color codes for prettier console output
const (
	colorReset  = "\033[0m"
	colorRed    = "\033[31m"
	colorGreen  = "\033[32m"
	colorYellow = "\033[33m"
	colorBlue   = "\033[34m"
	colorPurple = "\033[35m"
	colorCyan   = "\033[36m"
	colorGray   = "\033[37m"
)

// Logger struct to handle all logging operations
type Logger struct {
	ShowDebug bool
}

// NewLogger creates a new logger instance
func NewLogger(debug bool) *Logger {
	return &Logger{
		ShowDebug: debug,
	}
}

// createBox creates a boxed text display
func (l *Logger) createBox(title string, content []string) string {
	width := 80
	var sb strings.Builder

	// Calculate padding based on content
	padding := 2 // Minimum padding on each side
	maxContentLength := len(title)
	for _, line := range content {
		if len(line) > maxContentLength {
			maxContentLength = len(line)
		}
	}
	width = maxContentLength + (padding * 2) + 2 // +2 for borders

	// Top border with title
	sb.WriteString("╔═" + title + strings.Repeat("═", width-len(title)-3) + "╗\n")

	// Content
	for _, line := range content {
		paddingRight := width - len(line) - 3
		if paddingRight < 0 {
			paddingRight = 0
		}
		sb.WriteString("║ " + line + strings.Repeat(" ", paddingRight) + "║\n")
	}

	// Bottom border
	sb.WriteString("╚" + strings.Repeat("═", width-2) + "╝\n")

	return sb.String()
}

// FormatContainerConfig formats container configuration details
func (l *Logger) FormatContainerConfig(c container.Summary, ip string, sslConfig SSLConfig, hosts []string, redirects []Redirect, port string) string {
	var details []string

	// Container Info
	details = append(details, colorCyan+"Container:"+colorReset+" "+strings.TrimPrefix(c.Names[0], "/"))
	details = append(details, colorCyan+"Image:"+colorReset+"    "+c.Image)
	details = append(details, colorCyan+"Network:"+colorReset+"   "+ip+":"+port)

	// Hosts Configuration
	details = append(details, "")
	details = append(details, colorGreen+"Hosts:"+colorReset)
	for _, host := range hosts {
		details = append(details, "  • "+host)
	}

	// Redirects Configuration
	if len(redirects) > 0 {
		details = append(details, "")
		details = append(details, colorYellow+"Redirects:"+colorReset)
		for _, redirect := range redirects {
			details = append(details, "  • "+redirect.Source+" → "+redirect.Target)
		}
	}

	// SSL Configuration
	details = append(details, "")
	details = append(details, colorPurple+"SSL Configuration:"+colorReset)
	if sslConfig.Enabled {
		details = append(details, "  • Status:   "+colorGreen+"Enabled"+colorReset)
		details = append(details, "  • Provider: "+sslConfig.Provider)
		details = append(details, "  • Email:    "+sslConfig.Email)
	} else {
		details = append(details, "  • Status:   "+colorGray+"Disabled"+colorReset)
	}

	return l.createBox("📋 Container Configuration", details)
}

// CreateSummaryReport creates a summary of the configuration process
func (l *Logger) CreateSummaryReport(total, configured, problematic int, errors []string) string {
	var details []string

	// Statistics
	details = append(details, colorCyan+"Statistics:"+colorReset)
	details = append(details, fmt.Sprintf("  • Total Containers:      %d", total))
	details = append(details, fmt.Sprintf("  • Configured:           %s%d%s", colorGreen, configured, colorReset))
	if problematic > 0 {
		details = append(details, fmt.Sprintf("  • Problematic:          %s%d%s", colorRed, problematic, colorReset))
	}

	// Errors (if any)
	if len(errors) > 0 {
		details = append(details, "")
		details = append(details, colorRed+"Errors:"+colorReset)
		for _, err := range errors {
			details = append(details, "  • "+err)
		}
	}

	return l.createBox("📊 Configuration Summary", details)
}

// StartProcess logs the start of configuration generation
func (l *Logger) StartProcess() string {
	return l.createBox("🚀 proxma Configuration Generation",
		[]string{"Starting configuration generation process..."})
}

// Debug logs debug messages if debug mode is enabled
func (l *Logger) Debug(format string, v ...interface{}) {
	if l.ShowDebug {
		fmt.Printf(colorGray+"DEBUG: "+format+colorReset+"\n", v...)
	}
}

// Info logs information messages
func (l *Logger) Info(format string, v ...interface{}) {
	fmt.Printf(colorBlue+"INFO: "+format+colorReset+"\n", v...)
}

// Warning logs warning messages
func (l *Logger) Warning(format string, v ...interface{}) {
	fmt.Printf(colorYellow+"WARN: "+format+colorReset+"\n", v...)
}

// Error logs error messages
func (l *Logger) Error(format string, v ...interface{}) {
	fmt.Printf(colorRed+"ERROR: "+format+colorReset+"\n", v...)
}

// Success logs success messages
func (l *Logger) Success(format string, v ...interface{}) {
	fmt.Printf(colorGreen+"SUCCESS: "+format+colorReset+"\n", v...)
}
