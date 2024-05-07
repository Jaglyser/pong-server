use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    num::ParseFloatError,
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
                x: 400.,
                y: 300.,
                width: 20.,
                height: 20.,
                source: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1),
            };
            let velocity = Speed {
                dx: 1.,
                dy: 0.,
                source: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1),
            };
            self.render_components.push(ball);
            self.speed_components.push(velocity);
        }
    }

    fn create_player(&mut self, source: SocketAddr) -> &Renderable {
        let player = if self.render_components.len() == 0 {
            Renderable {
                x: 20.,
                y: 100.,
                width: 20.,
                height: 100.,
                source,
            }
        } else {
            Renderable {
                x: 760.,
                y: 100.,
                width: 20.,
                height: 100.,
                source,
            }
        };
        let speed = Speed {
            dx: 0.,
            dy: 0.,
            source,
        };
        self.render_components.push(player);
        self.speed_components.push(speed);

        return self.render_components.last().unwrap();
    }
}

struct Renderable {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    source: SocketAddr,
}

struct Speed {
    dy: f32,
    dx: f32,
    source: SocketAddr,
}

impl ToString for Renderable {
    fn to_string(&self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.width, self.height)
    }
}

struct CollisionSystem;

impl CollisionSystem {
    fn new() -> Self {
        CollisionSystem
    }

    fn collision(&mut self, world: &mut World) {
        if world.render_components.len() < 3 {
            return;
        }

        let (ball, speed) = world
            .render_components
            .iter()
            .zip(world.speed_components.iter_mut())
            .find(|(renderable, speed)| renderable.height == renderable.width 
                  && renderable.source == speed.source)
            .unwrap();


        let _collision = world
            .render_components
            .iter()
            .filter(|renderable| renderable.height != renderable.width)
            .find(|player| self.player_collision(player, ball))
            .map(|_| self.bounce(speed));
    }

    fn player_collision(&self, player: &Renderable, ball: &Renderable) -> bool {
        player.x <= ball.x + ball.width
        && player.x + player.width >= ball.x 
        && player.y <= ball.y 
        && player.y + player.height >= ball.y
    }

    fn ball_out_of_bounds(&mut self, world: &mut World) {
        world.render_components
            .iter_mut()
            .filter(|renderable| renderable.height == renderable.width)
            .for_each(|renderable| {
                self
                    .goal(renderable)
                    .then(|| self.new_ball(renderable));
            });
    }

    fn goal(&self, ball: &Renderable) -> bool {
        ball.x < 0. || ball.x > 800.
    }

    fn bounce(&mut self, velocity: &mut Speed) {
        velocity.dx = (-1.) * velocity.dx;
    }

    fn new_ball(&mut self, ball: &mut Renderable) {
        ball.x = 400.;
        ball.y = 300.;
    }

}

struct NetworkSystem {
    socket: UdpSocket,
    buf: [u8; 1024],
}

impl NetworkSystem {
    fn new() -> Self {
        let socket = UdpSocket::bind("0.0.0.0:8080").expect("Failed to bind to address");

        socket
            .set_nonblocking(true)
            .expect("Failed to set non-blocking");

        println!("Server listening on 0.0.0.0:8080");

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

    fn parse_player(&self, request: &str) -> Result<(f32, f32, f32, f32), ParseFloatError> {
        let parts: Vec<&str> = request.split_whitespace().collect();
        let x = parts[0].parse::<f32>()?;
        let y = parts[1].parse::<f32>()?;
        let width = parts[2].parse::<f32>()?;
        let height = parts[3].parse::<f32>()?;
        Ok((x, y, width, height))
    }

    fn send_state(&self, world: &mut World) {
        if world.render_components.len() < 2 {
            return;
        }
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
                println!("Sending state: {}", state);
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

    fn update_players(&self, x: f32, y: f32, source: SocketAddr, world: &mut World) {
        println!("Updating player: {} {} {}", x, y, source);
        world
            .render_components
            .iter_mut()
            .find(|player| player.source == source)
            .map(|player| {
                player.x = x;
                player.y = y;
            });
    }

    fn update_ball(&self, world: &mut World) {
        if world.render_components.len() >= 2 {
            world
                .render_components
                .iter_mut()
                .zip(world.speed_components.iter_mut())
                .find(|(renderable, speed)| renderable.height == renderable.width && renderable.source == speed.source)
                .map(|(renderable, speed)| {
                    renderable.x += speed.dx;
                    renderable.y += speed.dy; 
                });
        }
    }

    fn predict(&self, world: &mut World, dt: f32) {
        if world.render_components.len() >= 2 {
            world.render_components
                .iter_mut()
                .zip(world.speed_components.iter())
                .filter(|(r, s)| r.source == s.source)
                .for_each(|(r, s)| {
                    r.x += s.dx * dt;
                    r.y += s.dy * dt;
            });
        }
    }

}

fn main() {
    let mut world = World::new();
    let mut network_system = NetworkSystem::new();
    let mut collision_system = CollisionSystem::new();
    let control_system = ControlSystem::new();

    loop {
        collision_system.ball_out_of_bounds(&mut world);
        collision_system.collision(&mut world);
        control_system.update_ball(&mut world);

        if world.render_components.len() == 2 {
            world.create_ball();
        }

        let (size, source) = match network_system.receive() {
            Ok((size, source)) => (size, source),
            Err(_) => {
                continue
            },
        };

        let request = &network_system.parse_request(size);


        match network_system.handle_join(request, source, &mut world) {
            Ok(()) => {
                continue
            },
            Err(_) => {
            }
        }

        let (x, y) = match network_system.parse_player(request) {
            Ok((x, y, _width, _height)) => (x, y),
            Err(e) => {
                println!("Failed to parse player: {}", e);
                continue;
            }
        };

        control_system.update_players(x, y, source, &mut world);
        network_system.send_state(&mut world);
    }
}
