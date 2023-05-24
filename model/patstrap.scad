$fn=300;

SIZE=145;
STRENGTH=3;
WIDTH=15;
TOLERANCE=0.15;

module baseshape() {
    difference() {
        hull() {
            circle(d=SIZE);
            translate([-SIZE/2, 0, 0]) circle(d=SIZE/2);
        }
        hull() {
            circle(d=SIZE-STRENGTH*2);
            translate([-SIZE/2, 0, 0]) circle(d=SIZE/2-STRENGTH);
        }
    }
}


difference() {
    union() {
        translate([0, 0, WIDTH/4]) 
        linear_extrude(height = WIDTH/2, center = true, convexity = 3, scale=0.99)
        scale([1.01,1.01,1.01])
        baseshape();
        
        translate([0, 0, -WIDTH/4]) 
        linear_extrude(height = WIDTH/2, center = true, convexity = 3, scale=1.01)
        baseshape();
    }
    
    // remove end of circle to put head through
    translate([-SIZE, 0, 0]) 
    cube([SIZE, SIZE, SIZE], true);

    for (i = [-1,1]) {
        
        // make it get smaller at the end
        rotate([0,-2*i,0]) 
        translate([0, 0, (WIDTH/2+2)*i]) 
        cube([SIZE*2, SIZE*2, 10], true);

        
        // Hole for vibrator
        rotate([0, 0, 30 * i]) 
        translate([SIZE/2-2, 0, 0]) 
        rotate([0, 90, 0]) {
            cylinder(d=10+TOLERANCE, h=2.6, center=true);
            translate([0, -7*i, 3]) rotate([i*50, 0, 0]) cylinder(d=3, h=20, center=true);
        }
    }
}