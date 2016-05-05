class Node:
    def __init__(self, level):
        self.level = level
    
    def branch_generator(self):
        yield None

class Knapsack:
    def __init__(self, items, size):
        self.size = size
        self.items = items
    