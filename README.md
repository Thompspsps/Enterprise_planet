# Enterprise

Welcome aboard! We are glad to present you the planet Enterprise, part of professor Patrignani's Advanced Programming project.

This planet has the following characteristics:
* Type C
* Carbon generation
* Unbounded combination rules
* One energy cell
* One rocket

## Planet Behaviour

The behavior of the Enterprise varies depending on whether or not there are explorers currently on the planet.

### No Explorers on the Planet
When no explorers are present, the planet focuses on self-defense. Upon receiving a sunray, it will charge its energy cell and immediately use the stored energy to build a rocket for protection. The idea is that, by the time explorers arrive, the planet will already be protected against any incoming asteroids.

### Explorers on the planet
When explorers are on the planet, the planet will still charge its energy cell upon receiving a sunray. However, it will not immediately build a rocket. Instead, the energy will be conserved for the explorers' needs, except in the event of an incoming asteroid, where the planet will prioritize defense.

### Additional Behaviours

#### Minimizing Wasted Energy
If a sunray arrives while the energy cell is already charged, the planet will attempt to build a rocket to free up the cell, allowing it to charge with the incoming sunray.

#### Emergency Defense
If an asteroid is approaching and the planet doesn’t have a rocket, it will use the available energy to construct an emergency rocket, prioritizing the planet’s survival at all costs.

## Add Enterprise as a Dependency

In order to add Enterprise as a dependency, write the following lines on Cargo.toml.
```
[dependencies]
enterprise = {git = "https://github.com/Thompspsps/Enterprise_planet.git"}
```

## Creating the Planet
```
enterprise::create_planet(id, rx_orchestrator, tx_orchestrator, rx_explorer)
```

## Client Support

You can contact us in our [Telegram group chat](https://t.me/+IJlWkyHqlq9mOWJk).
