export interface DirectorySnippet {
  name: string;
  command: string;
  description: string;
}

export interface SnippetCategory {
  name: string;
  icon: string;
  items: DirectorySnippet[];
}

export const snippetDirectory: SnippetCategory[] = [
  {
    name: "System Info",
    icon: "mdi:monitor-dashboard",
    items: [
      {
        name: "OS Info",
        command: "cat /etc/os-release",
        description: "Display the operating system name, version, and distribution details.",
      },
      {
        name: "Hostname",
        command: "hostname -f",
        description: "Show the fully qualified domain name (FQDN) of the server.",
      },
      {
        name: "Uptime",
        command: "uptime",
        description: "Show how long the server has been running, number of users, and load averages.",
      },
      {
        name: "Kernel Version",
        command: "uname -a",
        description: "Display the Linux kernel version, CPU architecture, and system info.",
      },
      {
        name: "CPU Info",
        command: "lscpu | head -20",
        description: "Show processor details: model, core count, speed, and architecture.",
      },
      {
        name: "Memory Info",
        command: "free -h",
        description: "Display RAM and swap usage in human-readable format (MB/GB).",
      },
      {
        name: "Disk Usage",
        command: "df -h",
        description: "Show disk usage for each mounted partition in human-readable format.",
      },
      {
        name: "Disk Usage (sorted)",
        command: "du -sh /* 2>/dev/null | sort -rh | head -15",
        description: "List the 15 largest top-level directories, sorted by size.",
      },
    ],
  },
  {
    name: "Process & Resource",
    icon: "mdi:chart-bar",
    items: [
      {
        name: "Top Processes (CPU)",
        command: "ps aux --sort=-%cpu | head -15",
        description: "List the 15 processes consuming the most CPU.",
      },
      {
        name: "Top Processes (Memory)",
        command: "ps aux --sort=-%mem | head -15",
        description: "List the 15 processes consuming the most memory.",
      },
      {
        name: "Process Tree",
        command: "pstree -p | head -30",
        description: "Display running processes as a tree with their PIDs.",
      },
      {
        name: "Kill Process by Name",
        command: "pkill -f PROCESS_NAME",
        description: "Terminate all processes matching PROCESS_NAME. Replace with the actual name.",
      },
      {
        name: "IO Stats",
        command: "iostat -x 1 3",
        description: "Show detailed disk I/O statistics, sampled 3 times at 1-second intervals.",
      },
      {
        name: "Open Files",
        command: "lsof | wc -l",
        description: "Count the total number of open files across all processes.",
      },
    ],
  },
  {
    name: "Network",
    icon: "mdi:web",
    items: [
      {
        name: "IP Address",
        command: "ip addr show | grep 'inet '",
        description: "Show all active IP addresses assigned to network interfaces.",
      },
      {
        name: "Public IP",
        command: "curl -s ifconfig.me",
        description: "Fetch the server's public IP address using an external service.",
      },
      {
        name: "Open Ports",
        command: "ss -tlnp",
        description: "List all TCP ports in LISTEN state with the process using each port.",
      },
      {
        name: "Active Connections",
        command: "ss -tnp",
        description: "Show all active TCP connections and which processes own them.",
      },
      {
        name: "DNS Lookup",
        command: "dig +short example.com",
        description: "Resolve a domain name to its IP address. Replace example.com as needed.",
      },
      {
        name: "Ping Test",
        command: "ping -c 4 8.8.8.8",
        description: "Send 4 ICMP packets to Google DNS to test network connectivity.",
      },
      {
        name: "Route Table",
        command: "ip route",
        description: "Display the server's network routing table.",
      },
      {
        name: "Bandwidth Monitor",
        command: "iftop -t -s 5 2>/dev/null || echo 'Install: apt install iftop'",
        description: "Show real-time bandwidth usage per connection (requires iftop).",
      },
    ],
  },
  {
    name: "Logs",
    icon: "mdi:text-box-outline",
    items: [
      {
        name: "System Log (live)",
        command: "tail -f /var/log/syslog 2>/dev/null || tail -f /var/log/messages",
        description: "Stream system log entries in real time. Press Ctrl+C to stop.",
      },
      {
        name: "Auth Log (live)",
        command: "tail -f /var/log/auth.log 2>/dev/null || tail -f /var/log/secure",
        description: "Stream authentication events (logins, sudo, SSH) in real time.",
      },
      {
        name: "Last 50 Log Lines",
        command: "journalctl -n 50 --no-pager",
        description: "Show the 50 most recent entries from the systemd journal.",
      },
      {
        name: "Failed SSH Logins",
        command: "journalctl -u sshd --no-pager | grep 'Failed' | tail -20",
        description: "List the 20 most recent failed SSH login attempts.",
      },
      {
        name: "Nginx Error Log",
        command: "tail -50 /var/log/nginx/error.log 2>/dev/null || echo 'Nginx not found'",
        description: "Show the last 50 lines of the Nginx error log.",
      },
      {
        name: "Nginx Access Log",
        command: "tail -50 /var/log/nginx/access.log 2>/dev/null || echo 'Nginx not found'",
        description: "Show the last 50 lines of the Nginx access log.",
      },
    ],
  },
  {
    name: "Services",
    icon: "mdi:cog-outline",
    items: [
      {
        name: "List Running Services",
        command: "systemctl list-units --type=service --state=running",
        description: "Show all systemd services that are currently running.",
      },
      {
        name: "List Failed Services",
        command: "systemctl --failed",
        description: "Show services that have failed to start or crashed.",
      },
      {
        name: "Service Status",
        command: "systemctl status SERVICE_NAME",
        description: "Show detailed status of a specific service. Replace SERVICE_NAME.",
      },
      {
        name: "Restart Service",
        command: "sudo systemctl restart SERVICE_NAME",
        description: "Restart a service. Replace SERVICE_NAME (e.g. nginx, mysql, docker).",
      },
      {
        name: "Service Logs",
        command: "journalctl -u SERVICE_NAME -n 50 --no-pager",
        description: "Show the last 50 log entries for a specific service.",
      },
    ],
  },
  {
    name: "Docker",
    icon: "mdi:docker",
    items: [
      {
        name: "Running Containers",
        command: "docker ps --format 'table {{.Names}}\\t{{.Status}}\\t{{.Ports}}'",
        description: "List running Docker containers in a clean table format.",
      },
      {
        name: "All Containers",
        command: "docker ps -a --format 'table {{.Names}}\\t{{.Status}}\\t{{.Image}}'",
        description: "List all containers including stopped ones.",
      },
      {
        name: "Docker Images",
        command: "docker images --format 'table {{.Repository}}\\t{{.Tag}}\\t{{.Size}}'",
        description: "List all Docker images stored on the server.",
      },
      {
        name: "Container Logs",
        command: "docker logs --tail 50 CONTAINER_NAME",
        description: "Show the last 50 log lines from a specific container.",
      },
      {
        name: "Docker Disk Usage",
        command: "docker system df",
        description: "Show disk space used by Docker images, containers, and volumes.",
      },
      {
        name: "Docker Cleanup",
        command: "docker system prune -f",
        description: "Remove unused Docker resources (stopped containers, dangling images).",
      },
    ],
  },
  {
    name: "Files & Search",
    icon: "mdi:folder-search-outline",
    items: [
      {
        name: "Find Large Files",
        command: "find / -type f -size +100M -exec ls -lh {} \\; 2>/dev/null | head -20",
        description: "Search for files larger than 100MB across the entire server.",
      },
      {
        name: "Find Recent Files",
        command: "find /var -type f -mmin -30 2>/dev/null | head -20",
        description: "Find files modified in the last 30 minutes under /var.",
      },
      {
        name: "Search in Files",
        command: "grep -rn 'SEARCH_TERM' /path/to/dir",
        description: "Search for text inside files recursively. Replace SEARCH_TERM and path.",
      },
      {
        name: "File Permissions",
        command: "ls -la",
        description: "List all files including hidden ones with permissions and ownership.",
      },
      {
        name: "Folder Size",
        command: "du -sh */",
        description: "Show the size of each subfolder in the current directory.",
      },
    ],
  },
  {
    name: "Security",
    icon: "mdi:shield-lock-outline",
    items: [
      {
        name: "Who is Logged In",
        command: "w",
        description: "Show who is currently logged into the server and what they are doing.",
      },
      {
        name: "Login History",
        command: "last -20",
        description: "Display the 20 most recent login sessions.",
      },
      {
        name: "Failed Login Attempts",
        command: "lastb -20 2>/dev/null || echo 'Requires root'",
        description: "Show the 20 most recent failed login attempts (requires root).",
      },
      {
        name: "Firewall Rules (UFW)",
        command: "sudo ufw status verbose",
        description: "Display active UFW firewall rules and their status.",
      },
      {
        name: "Firewall Rules (iptables)",
        command: "sudo iptables -L -n --line-numbers",
        description: "List all iptables firewall rules with line numbers.",
      },
      {
        name: "Check Open Ports (external)",
        command: "ss -tlnp | grep LISTEN",
        description: "Show all open ports and the processes listening on them.",
      },
    ],
  },
  {
    name: "Package Manager",
    icon: "mdi:package-variant-closed",
    items: [
      {
        name: "Update Package List",
        command: "sudo apt update && apt list --upgradable 2>/dev/null || sudo yum check-update",
        description: "Check for available package updates (supports apt and yum).",
      },
      {
        name: "Upgrade All Packages",
        command: "sudo apt upgrade -y 2>/dev/null || sudo yum update -y",
        description: "Install all available package updates.",
      },
      {
        name: "Search Package",
        command: "apt search PACKAGE_NAME 2>/dev/null || yum search PACKAGE_NAME",
        description: "Search for a package by name (supports apt and yum).",
      },
      {
        name: "Installed Packages",
        command: "dpkg -l | tail -30 2>/dev/null || rpm -qa | tail -30",
        description: "List the 30 most recently installed packages.",
      },
    ],
  },
  {
    name: "Cron & Scheduled",
    icon: "mdi:clock-outline",
    items: [
      {
        name: "List Cron Jobs",
        command: "crontab -l 2>/dev/null || echo 'No crontab for current user'",
        description: "Show all scheduled cron jobs for the current user.",
      },
      {
        name: "All User Cron Jobs",
        command: "for user in $(cut -f1 -d: /etc/passwd); do echo \"=== $user ===\"; crontab -u $user -l 2>/dev/null; done",
        description: "List cron jobs for every user on the server (requires root).",
      },
      {
        name: "System Cron",
        command: "ls -la /etc/cron.d/ && cat /etc/crontab",
        description: "Show system-level cron jobs and the crontab configuration.",
      },
    ],
  },
];
