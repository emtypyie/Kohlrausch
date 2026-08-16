from machine import Pin as p
import time

# Define columns
columns = [p(7, p.IN), p(15, p.IN), p(16, p.IN)]

# Define rows
rows = [p(4, p.OUT), p(5, p.OUT), p(6, p.OUT)]

# Track current key state for debouncing
key_state = [[False]*3 for _ in range(3)]

while True:
    pressed_keys = []
    
    # Scan all rows
    for row_idx, row_pin in enumerate(rows):
        row_pin.value(0)  # Activate row
        time.sleep(0.001)  # Short delay for capacitance settling
        
        for col_idx, col_pin in enumerate(columns):
            if col_pin.value() == 0:  # Key pressed
                pressed_keys.append(f"r{row_idx+1}c{col_idx+1}")
        
        row_pin.value(1)  # Deactivate row
    
    # Print all pressed keys together (enables simultaneous detection)
    if pressed_keys:
        print(pressed_keys)
    
    time.sleep(0.01)  # Debounce delay
