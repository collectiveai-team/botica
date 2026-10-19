import logging

log = logging.getLogger(__name__)


def fetch(key):
    log.info("cache miss %s", key)
