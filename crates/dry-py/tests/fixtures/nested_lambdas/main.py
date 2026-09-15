def with_closures():
    left = lambda n: (
        (0 - n if n < 0 else n) + 1
        if True
        else 0
    )
    right = lambda n: (
        (0 - n if n < 0 else n) + 1
        if True
        else 0
    )
    return left(3) + right(4)
