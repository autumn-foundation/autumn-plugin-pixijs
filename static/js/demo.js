// Demo: custom code with the pixi:ready handle. It adds a ball that
// bounces. The ball uses `handle.motion`, so it stops for reduced motion
// and off screen, like the declared motion.
document.addEventListener("pixi:ready", (event) => {
  if (event.target.id !== "custom") return;
  const { PIXI, root, motion, size, update } = event.detail;
  const [width, height] = size;
  const ball = new PIXI.Graphics().circle(0, 0, 20).fill(0xf97316);
  ball.label = "ball";
  ball.position.set(width / 2, height / 2);
  root.addChild(ball);
  const velocity = { x: 160, y: 120 };
  motion.push((dt) => {
    ball.x += velocity.x * dt;
    ball.y += velocity.y * dt;
    if (ball.x < 20 || ball.x > width - 20) velocity.x *= -1;
    if (ball.y < 20 || ball.y > height - 20) velocity.y *= -1;
  });
  update();
});
