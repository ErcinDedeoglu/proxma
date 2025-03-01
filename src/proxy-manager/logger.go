package main

import (
	"fmt"
	"strings"
	"time"

	"github.com/docker/docker/api/types/container"
)

const (
	prefixInfo    = "[INFO] "
	prefixSuccess = "[OK] "
	prefixWarning = "[WARN] "
	prefixError   = "[ERROR] "
	prefixDebug   = "[DEBUG] "
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

// Basic logging methods
func (l *Logger) Debug(format string, v ...interface{}) {
	if l.ShowDebug {
		fmt.Printf(prefixDebug+format+"\n", v...)
	}
}

func (l *Logger) Info(format string, v ...interface{}) {
	fmt.Printf(prefixInfo+format+"\n", v...)
}

func (l *Logger) Warning(format string, v ...interface{}) {
	fmt.Printf(prefixWarning+format+"\n", v...)
}

func (l *Logger) Error(format string, v ...interface{}) {
	fmt.Printf(prefixError+format+"\n", v...)
}

func (l *Logger) Success(format string, v ...interface{}) {
	fmt.Printf(prefixSuccess+format+"\n", v...)
}

// logSection prints a section header
func (l *Logger) logSection(title string) {
	fmt.Printf("\n=== %s ===\n", title)
}

// StartProcess logs the start of configuration generation
func (l *Logger) StartProcess() {
	l.logSection("proxma Configuration Generation")
	l.Info("Version: %s", getVersion())
	l.Info("Time: %s", time.Now().Format("2006-01-02 15:04:05"))
	fmt.Println()
}

// LogContainerConfig logs container configuration details
func (l *Logger) LogContainerConfig(c container.Summary, ip string, sslConfig SSLConfig, hosts []string, redirects []Redirect, port string) {
	containerName := strings.TrimPrefix(c.Names[0], "/")

	// Container header
	l.Info("📦 Container: %s", containerName)
	l.Info("  🔌 Network: %s:%s", ip, port)

	// Domains/Hosts
	if len(hosts) > 0 {
		l.Info("  🌐 Domains: %s", strings.Join(hosts, ", "))
	}

	// Redirects
	if len(redirects) > 0 {
		redirectStrs := make([]string, len(redirects))
		for i, r := range redirects {
			redirectStrs[i] = fmt.Sprintf("%s ➡️  %s", r.Source, r.Target)
		}
		l.Info("  🔄 Redirects: %s", strings.Join(redirectStrs, ", "))
	}

	// SSL Configuration
	if sslConfig.Enabled {
		l.Info("  🔒 SSL: enabled (%s)", sslConfig.Provider)
		if sslConfig.Email != "" {
			l.Info("  📧 SSL Email: %s", sslConfig.Email)
		}
	}
}

func (l *Logger) LogSkippedContainer(containerName string, reason string) {
	if l.ShowDebug {
		l.Debug("⏭️  Skipped %s: %s", containerName, reason)
	}
}

// LogChanges logs configuration changes
func (l *Logger) LogChanges(added, removed []string) {
	if len(added) > 0 || len(removed) > 0 {
		l.logSection("Configuration Changes")
	}

	if len(added) > 0 {
		l.Success("Added configurations:")
		for _, a := range added {
			l.Info("  + %s", a)
		}
	}

	if len(removed) > 0 {
		l.Info("Removed configurations:")
		for _, r := range removed {
			l.Info("  - %s", r)
		}
	}
}

// LogSummary logs configuration summary
func (l *Logger) LogSummary(total, configured, problematic int, errors []string) {
	l.logSection("📊 Summary")

	// Overall stats
	l.Info("Total containers: %d", total)

	// Configured containers
	if configured > 0 {
		l.Success("✅ Configured containers: %d", configured)
	}

	// Skipped containers
	skipped := total - configured
	if skipped > 0 {
		l.Info("⏭️  Skipped containers: %d", skipped)
	}

	// Problems section
	if problematic > 0 {
		l.Warning("⚠️  Problems found: %d", problematic)
		for _, err := range errors {
			l.Error("  • %s", err)
		}
	}
}

// LogInitialization logs initialization status
func (l *Logger) LogInitialization(action, path string) {
	if l.ShowDebug {
		l.Debug("%s: %s", action, path)
	}
}

// LogInitComplete logs completion of initialization
func (l *Logger) LogInitComplete() {
	l.Success("Initialization completed")
	fmt.Println()
}

// LogWatcherStatus logs Docker events watcher status
func (l *Logger) LogWatcherStatus(status string) {
	if status == "start" {
		l.Info("Starting Docker events watcher...")
	} else if status == "ready" {
		l.Success("Docker events watcher started")
	}
}

// LogConfigTest logs nginx configuration test results
func (l *Logger) LogConfigTest(success bool, output string) {
	if success {
		l.Success("Nginx configuration test passed")
	} else {
		l.Error("Nginx configuration test failed: %s", output)
	}
}

// LogReload logs nginx reload status
func (l *Logger) LogReload(success bool, err error) {
	if success {
		l.Success("Nginx configuration reloaded")
	} else {
		l.Error("Failed to reload Nginx: %v", err)
	}
}
