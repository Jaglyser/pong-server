use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    num::ParseIntError,
    slice::IterMut,
    time::Instant,
};

struct World {
    render_components: Vec<Renderable>,
    speed_components: Vec<Speed>,
}

impl World {
    fn new() -> Self {
        World {
            render_components: Vec::new(),
            speed_components: Vec::new(),
        }
    }

    fn create_ball(&mut self) {
        if self.render_components.len() == 2 {
            let ball = Renderable {
                x: 400,
                y: 300,
                width: 20,
                height: 20,
                source: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1),
            };
            let velocity = Speed {
                dx: 1,
                dy: 0,
                last_update: Instant::now(),
                source: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1),
            };
            self.render_components.push(ball);
            self.speed_components.push(velocity);
        }
    }

    fn create_player(&mut self, source: SocketAddr) -> &Renderable {
        let player = if self.render_components.len() == 0 {
            Renderable {
                x: 20,
                y: 100,
                width: 20,
                height: 100,
                source,
            }
        } else {
            Renderable {
                x: 760,
                y: 100,
                width: 20,
                height: 100,
                source,
            }
        };
        let speed = Speed {
            dx: 0,
            dy: 0,
            last_update: Instant::now(),
            source,
        };
        self.render_components.push(player);
        self.speed_components.push(speed);

        return self.render_components.last().unwrap();
    }
}

struct Renderable {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    source: SocketAddr,
}

struct Speed {
    dy: i32,
    dx: i32,
    last_update: Instant,
    source: SocketAddr,
}

impl ToString for Renderable {
    fn to_string(&self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.width, self.height)
    }
}

struct CollisionSystem {
    ball: Renderable,
    velocity: Speed,
}

impl CollisionSystem {
    fn new() -> Self {
        CollisionSystem {
            ball: Renderable {
                x: 400,
                y: 300,
                width: 20,
                height: 20,
                source: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            },
            velocity: Speed {
                dx: 1,
                dy: 1,
                last_update: Instant::now(),
                source: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            },
        }
    }

    fn detect_collision(&mut self, render_components: &mut IterMut<Renderable>) {
        if render_components.len() >= 2 {
            render_components
                .find(|player| self.player_collision(player))
                .map(|_| self.bounce());
        }
    }

    fn player_collision(&self, player: &Renderable) -> bool {
        player.height != player.width
            && self.ball.x < player.x + player.width
            && self.ball.x + self.ball.width > player.x
            && self.ball.y < player.y + player.height
            && self.ball.y + self.ball.height > player.y
    }

    fn goal(&self) -> bool {
        self.ball.x < 0 || self.ball.x > 800
    }

    fn bounce(&mut self) {
        self.velocity.dx = -1 * self.velocity.dx;
    }

    fn new_ball(&mut self, render_components: &mut IterMut<Renderable>) {
        render_components
            .find(|renderable| renderable.height == renderable.width)
            .map(|r| {
                r.x = 400;
                r.y = 300;
                //r.dx = 1;
                //r.dy = 1;
            });
    }
}

struct NetworkSystem {
    socket: UdpSocket,
    buf: [u8; 1024],
}

impl NetworkSystem {
    fn new() -> Self {
        let socket = UdpSocket::bind("127.0.0.1:8080").expect("Failed to bind to address");

        socket
            .set_nonblocking(true)
            .expect("Failed to set non-blocking");

        println!("Server listening on 127.0.0.1:8080");

        NetworkSystem {
            socket,
            buf: [0; 1024],
        }
    }

    fn receive(&mut self) -> Result<(usize, SocketAddr), std::io::Error> {
        self.socket.recv_from(&mut self.buf)
    }

    fn parse_request(&self, size: usize) -> &str {
        std::str::from_utf8(&self.buf[..size]).unwrap()
    }

    fn handle_join(
        &self,
        request: &str,
        source: SocketAddr,
        world: &mut World,
    ) -> Result<(), std::io::Error> {
        if request == "join" && world.render_components.len() < 3 {
            let player = world.create_player(source);
            let response = format!(
                "{} {} {} {}",
                player.x, player.y, player.width, player.height
            );
            self.socket
                .send_to(response.as_bytes(), source)
                .expect("Failed to send response");
            println!(
                "Player joined! Total players: {}",
                world.render_components.len()
            );
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Game is full",
            ))
        }
    }

    fn parse_player(&self, request: &str) -> Result<(i32, i32, i32, i32), ParseIntError> {
        let parts: Vec<&str> = request.split_whitespace().collect();
        let x = parts[0].parse::<i32>()?;
        let y = parts[1].parse::<i32>()?;
        let width = parts[2].parse::<i32>()?;
        let height = parts[3].parse::<i32>()?;
        Ok((x, y, width, height))
    }

    fn send_state(&self, world: &World) {
        println!("Sending state");
        world
            .render_components
            .iter()
            .filter(|renderable| renderable.height != renderable.width)
            .for_each(|renderable| {
                let state = world
                    .render_components
                    .iter()
                    .filter(|r| r.source != renderable.source)
                    .map(|r| r.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");
                self.socket
                    .send_to(state.as_bytes(), renderable.source)
                    .expect("Failed to send response");
            });
    }
}

struct ControlSystem;

impl ControlSystem {
    fn new() -> Self {
        ControlSystem
    }

    fn update(&self, x: i32, y: i32, source: SocketAddr, world: &mut World) {
        world
            .render_components
            .iter_mut()
            .find(|player| player.source == source)
            .map(|player| {
                player.x = x;
                player.y = y;
            });
        if world.render_components.len() >= 2 {
            world
                .render_components
                .iter_mut()
                .zip(world.speed_components.iter())
                .find(|(renderable, speed)| {
                    renderable.height == renderable.width && renderable.source == speed.source
                })
                .map(|(renderable, speed)| {
                    renderable.x += speed.dx;
                    renderable.y += speed.dy;
                });
        }
    }
}

fn main() {
    let mut world = World::new();
    let mut network_system = NetworkSystem::new();
    let mut collision_system = CollisionSystem::new();
    let control_system = ControlSystem::new();
    let mut start = Instant::now();

    loop {
        if start.elapsed().as_secs() < (1 / 60) {
            continue;
        }

        if world.render_components.len() == 2 {
            world.create_ball();
            println!("Ball created!");
        }

        let (size, source) = match network_system.receive() {
            Ok((size, source)) => (size, source),
            Err(_) => continue,
        };

        println!("Received {} bytes from {}", size, source);
        let request = &network_system.parse_request(size);

        match network_system.handle_join(request, source, &mut world) {
            Ok(()) => continue,
            Err(_) => {}
        }

        let (x, y, _width, _height) = match network_system.parse_player(request) {
            Ok((x, y, width, height)) => (x, y, width, height),
            Err(_) => {
                continue;
            }
        };

        control_system.update(x, y, source, &mut world);
        collision_system.detect_collision(&mut world.render_components.iter_mut());
        collision_system
            .goal()
            .then(|| collision_system.new_ball(&mut world.render_components.iter_mut()));
        network_system.send_state(&world);

        start = Instant::now();
    }
}
