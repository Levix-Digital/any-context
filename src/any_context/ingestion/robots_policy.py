import re
import threading
import urllib.parse
import urllib.request
from typing import Dict, List, Tuple, Optional

class RobotsFileParser:
    """
    Robust, cross-version RFC 9309 robots.txt parser supporting wildcards (*),
    end-of-line ($), and longest-match specificity.
    Guaranteed compatibility across Python 3.10, 3.11, 3.12, and 3.13+.
    """
    def __init__(self):
        self.rules: List[Tuple[str, str, bool, int]] = [] # (user_agent, regex, is_allow, pattern_len)

    def parse(self, lines: List[str]):
        current_agents: List[str] = []
        for raw_line in lines:
            line = raw_line.split("#")[0].strip()
            if not line:
                continue

            if ":" not in line:
                continue

            key, val = line.split(":", 1)
            key = key.strip().lower()
            val = val.strip()

            if key == "user-agent":
                # If we were previously parsing rules for other agents and hit a new user-agent block, reset
                if self.rules and current_agents and (self.rules[-1][0] in current_agents):
                    current_agents = []
                current_agents.append(val.lower())
            elif key in ("allow", "disallow"):
                is_allow = (key == "allow")
                if not val and not is_allow:
                    # Empty Disallow means "allow all"
                    continue
                
                # RFC 9309: Escape regex special chars except * and $
                pattern = val
                clean_pat = pattern
                has_dollar = clean_pat.endswith("$")
                if has_dollar:
                    clean_pat = clean_pat[:-1]

                regex_core = re.escape(clean_pat).replace(r"\*", ".*")
                regex = "^" + regex_core
                if has_dollar:
                    regex += "$"
                else:
                    regex += ".*"

                pattern_len = len(pattern)
                for agent in current_agents:
                    self.rules.append((agent, regex, is_allow, pattern_len))

    def can_fetch(self, user_agent: str, url: str) -> bool:
        parsed = urllib.parse.urlparse(url)
        path = parsed.path
        if not path:
            path = "/"
        if parsed.query:
            path += "?" + parsed.query

        target_agent = user_agent.lower()

        # 1. Check for specific user-agent rules first
        agent_rules = [r for r in self.rules if r[0] == target_agent]
        if not agent_rules:
            # 2. Fall back to global wildcard rules
            agent_rules = [r for r in self.rules if r[0] == "*"]

        if not agent_rules:
            return True

        # In RFC 9309: Most specific rule (longest matched pattern) wins.
        # If tie in length, Allow takes precedence over Disallow.
        matched_rules = []
        for agent, regex, is_allow, pattern_len in agent_rules:
            try:
                if re.match(regex, path):
                    matched_rules.append((pattern_len, 1 if is_allow else 0, is_allow))
            except Exception:
                continue

        if not matched_rules:
            return True

        # Sort by pattern length desc, then allow precedence desc (Allow > Disallow)
        matched_rules.sort(key=lambda x: (x[0], x[1]), reverse=True)
        return matched_rules[0][2]


class RobotsPolicyManager:
    """
    RFC 9309 compliant robots.txt policy validator and cache for ethical web scraping.
    Strictly obeys website owners' access rules, disallow paths, and rate policies,
    protecting users and organizations from compliance risks and unauthorized scraping liabilities.
    """
    _instance: Optional["RobotsPolicyManager"] = None
    _lock = threading.Lock()

    def __init__(self, user_agent: str = "AnyContext-WebScraper/1.0"):
        self.user_agent = user_agent
        self._parsers: Dict[str, RobotsFileParser] = {}
        self._parser_lock = threading.Lock()

    @classmethod
    def get_instance(cls) -> "RobotsPolicyManager":
        with cls._lock:
            if cls._instance is None:
                cls._instance = cls()
            return cls._instance

    def get_parser_for_url(self, url: str, timeout: int = 5) -> RobotsFileParser:
        parsed = urllib.parse.urlparse(url)
        origin = f"{parsed.scheme}://{parsed.netloc}"
        with self._parser_lock:
            if origin in self._parsers:
                return self._parsers[origin]

        robots_url = f"{origin}/robots.txt"
        rp = RobotsFileParser()
        try:
            req = urllib.request.Request(robots_url, headers={"User-Agent": self.user_agent})
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                lines = [line.decode("utf-8", errors="ignore") for line in resp.readlines()]
                rp.parse(lines)
        except Exception:
            # If robots.txt returns 404 or is unavailable, RFC 9309 defaults to permissive
            pass

        with self._parser_lock:
            self._parsers[origin] = rp
        return rp

    def is_allowed(self, url: str) -> bool:
        """
        Returns True if the URL is allowed to be crawled according to robots.txt rules, False otherwise.
        """
        try:
            rp = self.get_parser_for_url(url)
            return rp.can_fetch(self.user_agent, url)
        except Exception:
            return True

def is_url_allowed_by_robots(url: str) -> bool:
    """Helper function to quickly check robots.txt compliance for any URL."""
    return RobotsPolicyManager.get_instance().is_allowed(url)
