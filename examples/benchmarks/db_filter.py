import sqlite3
import time

conn = sqlite3.connect(':memory:')
cursor = conn.cursor()

# Create table
cursor.execute('''
    CREATE TABLE records (
        id INTEGER PRIMARY KEY,
        balance INTEGER,
        status INTEGER
    )
''')

# Insert 1,000,000 rows
print("Populating DB with 1,000,000 rows...")
rows = [(i, (i * 17) % 5000, i % 2) for i in range(1000000)]
cursor.executemany("INSERT INTO records (id, balance, status) VALUES (?, ?, ?)", rows)
conn.commit()

print("Running SQL Filter...")
start = time.time()
# Filter condition: balance > 1000 AND status = 1
cursor.execute("SELECT COUNT(*) FROM records WHERE balance > 1000 AND status = 1")
count = cursor.fetchone()[0]
end = time.time()

print(f"Result Count: {count}")
print(f"Time taken: {end - start:.6f} seconds")
