import time

SIZE = 10000000

print(f"Creating 2 arrays of {SIZE} elements...")
a = [1] * SIZE
b = [2] * SIZE

print("Running Python List Vector Addition...")
start = time.time()
# Python list comprehension for element-wise addition
c = [x + y for x, y in zip(a, b)]
end = time.time()

print(f"Result length: {len(c)}")
print(f"Time taken: {end - start:.6f} seconds")
