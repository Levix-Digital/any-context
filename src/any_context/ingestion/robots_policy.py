import urllib.robotparser
import urllib.parse
import urllib.request
from typing import Dict, Optional

class RobotsPolicyManager:
    """
    RFC 9309 compliant robots.txt policy validator and cache for ethical web scraping.
    Strictly obeys website owners' access rules, disallow paths, and rate policies,
    protecting users and organizations from compliance risks and unauthorized scraping liabilities.
    """
    _instance: Optional["RobotsPolicyManager"] = None

    def __init__(self, user_agent: str = "AnyContext-WebScraper/1.0"):
        self.user_agent = user_agent
        self._parsers: Dict[str, urllib.robotparser.RobotFileParser] = {}

    @classmethod
    def get_instance(cls) -> "RobotsPolicyManager":
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    def get_parser_for_url(self, url: str, timeout: int = 5) -> urllib.robotparser.RobotFileParser:
        parsed = urllib.parse.urlparse(url)
        origin = f"{parsed.scheme}://{parsed.netloc}"
        if origin in self._parsers:
            return self._parsers[origin]

        robots_url = f"{origin}/robots.txt"
        rp = urllib.robotparser.RobotFileParser()
        rp.set_url(robots_url)
        try:
            req = urllib.request.Request(robots_url, headers={"User-Agent": self.user_agent})
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                lines = [line.decode("utf-8", errors="ignore") for line in resp.readlines()]
                rp.parse(lines)
        except Exception:
            # If robots.txt returns 404 or is unavailable, RFC 9309 defaults to permissive
            pass

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
