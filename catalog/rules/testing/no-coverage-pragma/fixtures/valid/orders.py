def total(order):
    for item in order.items:  # pragma: no branch
        yield item.price


if __name__ == "__main__":  # pragma: no cover  # botica: ignore[no-coverage-pragma] -- runs only as a script
    print(sum(total(load())))
