v {xschem version=3.4.7 file_version=1.2}
G {}
K {}
V {}
S {}
E {}
N -30 -160 60 -160 {lab=Vdd}
N 60 -160 150 -160 {lab=Vdd}
N -260 310 60 310 {lab=Vss}
N 280 160 280 310 {lab=Vss}
N 60 310 280 310 {lab=Vss}
N 280 20 280 100 {lab=out}
N 150 -160 280 -160 {lab=Vdd}
N 280 -10 380 -10 {lab=Vdd}
N 380 -160 380 -10 {lab=Vdd}
N 280 130 380 130 {lab=Vss}
N 380 130 380 310 {lab=Vss}
N 280 310 380 310 {lab=Vss}
N 280 -160 380 -160 {lab=Vdd}
N -220 -90 -220 -30 {lab=#net1}
N -260 -30 -220 -30 {lab=#net1}
N -350 -90 -260 -90 {lab=Vdd}
N -350 -160 -350 -90 {lab=Vdd}
N -350 -160 -30 -160 {lab=Vdd}
N -260 -160 -260 -120 {lab=Vdd}
N -100 0 -30 0 {lab=#net2}
N -100 0 -100 20 {lab=#net2}
N -30 0 60 0 {lab=#net2}
N 60 0 60 20 {lab=#net2}
N -100 80 -100 170 {lab=#net3}
N 60 80 60 170 {lab=#net4}
N -60 200 20 200 {lab=#net3}
N -20 150 -20 200 {lab=#net3}
N -100 150 -20 150 {lab=#net3}
N -220 -90 -70 -90 {lab=#net1}
N -30 -160 -30 -120 {lab=Vdd}
N -100 290 -100 310 {lab=Vss}
N 60 290 60 310 {lab=Vss}
N -260 -60 -260 40 {lab=#net1}
N -100 50 60 50 {lab=Vdd}
N -30 -90 30 -90 {lab=Vdd}
N 30 -160 30 -90 {lab=Vdd}
N 60 200 130 200 {lab=Vss}
N 130 200 130 310 {lab=Vss}
N -170 200 -100 200 {lab=Vss}
N -170 200 -170 310 {lab=Vss}
N 30 -90 30 50 {lab=Vdd}
N 60 130 240 130 {lab=#net4}
N 280 -160 280 -130 {lab=Vdd}
N 280 -70 280 -40 {lab=#net5}
N 240 60 280 60 {lab=out}
N 180 60 180 130 {lab=#net4}
N 280 60 430 60 {lab=out}
N -30 -60 -30 0 {lab=#net2}
N -260 40 -260 100 {lab=#net1}
N -100 230 -100 290 {lab=Vss}
N 60 230 60 290 {lab=Vss}
N 180 -10 240 -10 {lab=#net1}
N -100 -10 180 -10 {lab=#net1}
N -100 -90 -100 -10 {lab=#net1}
N -260 290 -260 310 {lab=Vss}
N -260 100 -260 230 {lab=#net1}
N -220 150 -220 260 {lab=#net1}
N -260 150 -220 150 {lab=#net1}
N -360 260 -260 260 {lab=Vss}
N -360 260 -360 310 {lab=Vss}
N -360 310 -260 310 {lab=Vss}
N -590 100 -540 100 {lab=GND}
N -540 0 -540 40 {lab=#net6}
N -540 160 -540 210 {lab=Vss}
N -540 -160 -350 -160 {lab=Vdd}
N -540 210 -540 310 {lab=Vss}
N -540 310 -360 310 {lab=Vss}
N -540 -100 -540 0 {lab=#net6}
N 100 50 100 340 {lab=Vmas}
N 430 -280 430 60 {lab=out}
N -140 -280 430 -280 {lab=out}
N -190 50 -140 50 {lab=out}
N -140 -280 -140 50 {lab=out}
C {sky130_fd_pr/corner.sym} -690 -360 0 0 {name=CORNER only_toplevel=true corner=tt}
C {code_shown.sym} 540 -400 0 0 {name=s1 only_toplevel=false value="
.save all
.param w1 = 26.0
.param w3 = 6.85
.param w5 = 0.5
.param w6 = 51.25
.param w7 = 1.8
.param w8 = 0.65
.param w9 = 0.5
.param l1 = 0.7
.param l6 = 0.7
.param l8 = 0.7
.control
  dc V1 -0.9 0.9 0.0001
  let error = (v(Vmas)-v(out))^2 * (1-v(Vmas))
  meas dc corriente_media AVG i(Vmeas)/i(Vmeas1)
  meas dc error_total AVG error
  wrdata fitness_data.txt corriente_media error_total
.endc
"}
C {ipin.sym} -350 -160 0 0 {name=p1 lab=Vdd}
C {ipin.sym} 100 50 0 1 {name=p3 lab=Vmas}
C {ipin.sym} 380 310 0 1 {name=p4 lab=Vss}
C {opin.sym} 430 60 0 0 {name=p7 lab=out}
C {sky130_fd_pr/nfet_01v8_lvt.sym} 260 130 0 0 {name=M10
W=\{w6\}
L=\{l6\}
nf=1
mult=10
ad="expr('int((@nf + 1)/2) * @W / @nf * 0.29')"
pd="expr('2*int((@nf + 1)/2) * (@W / @nf + 0.29)')"
as="expr('int((@nf + 2)/2) * @W / @nf * 0.29')"
ps="expr('2*int((@nf + 2)/2) * (@W / @nf + 0.29)')"
nrd="expr('0.29 / @W ')" nrs="expr('0.29 / @W ')"
sa=0 sb=0 sd=0
model=nfet_01v8_lvt
spiceprefix=X
}
C {sky130_fd_pr/pfet_01v8_lvt.sym} 260 -10 0 0 {name=M8
W=\{w7\}
L=\{l6\}
nf=1
mult=10
ad="expr('int((@nf + 1)/2) * @W / @nf * 0.29')"
pd="expr('2*int((@nf + 1)/2) * (@W / @nf + 0.29)')"
as="expr('int((@nf + 2)/2) * @W / @nf * 0.29')"
ps="expr('2*int((@nf + 2)/2) * (@W / @nf + 0.29)')"
nrd="expr('0.29 / @W ')" nrs="expr('0.29 / @W ')"
sa=0 sb=0 sd=0
model=pfet_01v8_lvt
spiceprefix=X
}
C {sky130_fd_pr/pfet_01v8_lvt.sym} -240 -90 0 1 {name=M7
W=\{w8\}
L=\{l8\}
nf=1
mult=1
ad="expr('int((@nf + 1)/2) * @W / @nf * 0.29')"
pd="expr('2*int((@nf + 1)/2) * (@W / @nf + 0.29)')"
as="expr('int((@nf + 2)/2) * @W / @nf * 0.29')"
ps="expr('2*int((@nf + 2)/2) * (@W / @nf + 0.29)')"
nrd="expr('0.29 / @W ')" nrs="expr('0.29 / @W ')"
sa=0 sb=0 sd=0
model=pfet_01v8_lvt
spiceprefix=X
}
C {sky130_fd_pr/nfet_01v8_lvt.sym} -80 200 0 1 {name=M3
W=\{w3\}
L=\{l1\}
nf=1
mult=1
ad="expr('int((@nf + 1)/2) * @W / @nf * 0.29')"
pd="expr('2*int((@nf + 1)/2) * (@W / @nf + 0.29)')"
as="expr('int((@nf + 2)/2) * @W / @nf * 0.29')"
ps="expr('2*int((@nf + 2)/2) * (@W / @nf + 0.29)')"
nrd="expr('0.29 / @W ')" nrs="expr('0.29 / @W ')"
sa=0 sb=0 sd=0
model=nfet_01v8_lvt
spiceprefix=X
}
C {sky130_fd_pr/nfet_01v8_lvt.sym} 40 200 0 0 {name=M4
W=\{w3\}
L=\{l1\}
nf=1
mult=1
ad="expr('int((@nf + 1)/2) * @W / @nf * 0.29')"
pd="expr('2*int((@nf + 1)/2) * (@W / @nf + 0.29)')"
as="expr('int((@nf + 2)/2) * @W / @nf * 0.29')"
ps="expr('2*int((@nf + 2)/2) * (@W / @nf + 0.29)')"
nrd="expr('0.29 / @W ')" nrs="expr('0.29 / @W ')"
sa=0 sb=0 sd=0
model=nfet_01v8_lvt
spiceprefix=X
}
C {sky130_fd_pr/pfet_01v8_lvt.sym} -50 -90 0 0 {name=M5
W=\{w5\}
L=\{l1\}
nf=1
mult=1
ad="expr('int((@nf + 1)/2) * @W / @nf * 0.29')"
pd="expr('2*int((@nf + 1)/2) * (@W / @nf + 0.29)')"
as="expr('int((@nf + 2)/2) * @W / @nf * 0.29')"
ps="expr('2*int((@nf + 2)/2) * (@W / @nf + 0.29)')"
nrd="expr('0.29 / @W ')" nrs="expr('0.29 / @W ')"
sa=0 sb=0 sd=0
model=pfet_01v8_lvt
spiceprefix=X
}
C {sky130_fd_pr/pfet_01v8_lvt.sym} -120 50 0 0 {name=M1
W=\{w1\}
L=\{l1\}
nf=1
mult=1
ad="expr('int((@nf + 1)/2) * @W / @nf * 0.29')"
pd="expr('2*int((@nf + 1)/2) * (@W / @nf + 0.29)')"
as="expr('int((@nf + 2)/2) * @W / @nf * 0.29')"
ps="expr('2*int((@nf + 2)/2) * (@W / @nf + 0.29)')"
nrd="expr('0.29 / @W ')" nrs="expr('0.29 / @W ')"
sa=0 sb=0 sd=0
model=pfet_01v8_lvt
spiceprefix=X
}
C {sky130_fd_pr/pfet_01v8_lvt.sym} 80 50 0 1 {name=M2
W=\{w1\}
L=\{l1\}
nf=1
mult=1
ad="expr('int((@nf + 1)/2) * @W / @nf * 0.29')"
pd="expr('2*int((@nf + 1)/2) * (@W / @nf + 0.29)')"
as="expr('int((@nf + 2)/2) * @W / @nf * 0.29')"
ps="expr('2*int((@nf + 2)/2) * (@W / @nf + 0.29)')"
nrd="expr('0.29 / @W ')" nrs="expr('0.29 / @W ')"
sa=0 sb=0 sd=0
model=pfet_01v8_lvt
spiceprefix=X
}
C {sky130_fd_pr/cap_mim_m3_2.sym} 210 60 1 0 {name=C1 model=cap_mim_m3_2 W=32 L=33 MF=1 spiceprefix=X}
C {sky130_fd_pr/cap_mim_m3_1.sym} 400 90 0 0 {name=C2 model=cap_mim_m3_1 W=70 L=70 MF=1 spiceprefix=X}
C {gnd.sym} 400 120 0 0 {name=l1 lab=GND}
C {sky130_fd_pr/nfet_01v8_lvt.sym} -240 260 0 1 {name=M6
W=\{w9\}
L=\{l8\}
nf=1
mult=1
ad="expr('int((@nf + 1)/2) * @W / @nf * 0.29')"
pd="expr('2*int((@nf + 1)/2) * (@W / @nf + 0.29)')"
as="expr('int((@nf + 2)/2) * @W / @nf * 0.29')"
ps="expr('2*int((@nf + 2)/2) * (@W / @nf + 0.29)')"
nrd="expr('0.29 / @W ')" nrs="expr('0.29 / @W ')"
sa=0 sb=0 sd=0
model=nfet_01v8_lvt
spiceprefix=X
}
C {vsource.sym} -540 70 0 0 {name=Vplus value=0.9 savecurrent=false}
C {vsource.sym} -540 130 0 0 {name=Vminus value=0.9 savecurrent=false}
C {gnd.sym} -590 100 0 0 {name=l2 lab=GND}
C {ammeter.sym} -540 -130 2 0 {name=Vmeas savecurrent=true spice_ignore=0}
C {vsource.sym} 100 370 0 0 {name=V1 value=0.9 savecurrent=false}
C {gnd.sym} 100 400 0 0 {name=l3 lab=GND}
C {ammeter.sym} 280 -100 0 0 {name=Vmeas1 savecurrent=true spice_ignore=0}
