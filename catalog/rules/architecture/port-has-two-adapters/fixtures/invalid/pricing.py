from typing import Protocol


class PricingStrategy(Protocol):
    def price(self, subtotal: int, quantity: int) -> int: ...


class StandardPricing:
    def price(self, subtotal: int, quantity: int) -> int:
        if quantity >= 10:
            return subtotal * 95 // 100
        return subtotal


def checkout(subtotal: int, quantity: int, pricing: PricingStrategy) -> int:
    return pricing.price(subtotal, quantity)
