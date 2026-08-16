from machine import Pin as p
import time
#columns:
c1=p(7,p.IN)
c2=p(15,p.IN)
c3=p(16,p.IN)
#rows:
r1=p(4,p.OUT)
r2=p(5,p.OUT)
r3=p(6,p.OUT)
#defining pins is done
#=========================mapping:==========================
while True:
    #---------------------scanning row1----------------------
    r1.value(0)
    r2.value(1)
    r3.value(1)
    if c1.value()==0:
        print("r1c1")
    if c2.value()==0:
        print("r1c2")
    if c3.value()==0:
        print("r1c3")
    r1.value(1)
    time.sleep(0.01)
    #---------------------scanning row2----------------------
    r1.value(1)
    r2.value(0)
    r3.value(1)
    if c1.value()==0:
        print("r2c1")
    if c2.value()==0:
        print("r2c2")
    if c3.value()==0:
        print("r2c3")
    r2.value(1)
    time.sleep(0.01)
    #---------------------scanning row3----------------------
    r1.value(1)
    r2.value(1)
    r3.value(0)
    if c1.value()==0:
        print("r3c1")
    if c2.value()==0:
        print("r3c2")
    if c3.value()==0:
        print("r3c3")
    r3.value(1)
    time.sleep(0.01)