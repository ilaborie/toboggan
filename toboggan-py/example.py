from time import sleep
from toboggan_py import Toboggan

tbg = Toboggan("localhost", 8080)

print(f"toboggan: {tbg}")
print(f"state: {tbg.state}")

tbg.previous()
sleep(1)
print(f"state after previous: {tbg.state}")

tbg.next()
sleep(1)
print(f"state after next: {tbg.state}")
