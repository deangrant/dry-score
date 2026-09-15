# Unique bodies so dry-py dogfood stays at findings=0.

def accumulate_scores(values):
    total = 0
    for value in values:
        if value > 0:
            total += value
    return total
