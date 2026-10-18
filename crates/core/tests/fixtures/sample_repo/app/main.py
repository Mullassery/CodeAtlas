from core.engine import start
from app.util import Service


def run():
    service = Service()
    service.perform()
    return start()
