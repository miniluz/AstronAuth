## Requirements

You will have to [install Nix](https://nixos.org/download/)
and [enable flakes](https://nixos.wiki/wiki/Flakes#Other_Distros.2C_without_Home-Manager),
install [direnv](https://direnv.net/#basic-installation),
read the [.envrc](./.envrc) file to see what it does,
read the [Nix flake](./flake.nix) to see what's installed,
and run `direnv allow` on the project directory.
On Code, you might want to install the [direnv extension](https://github.com/direnv/direnv-vscode).

If you plan on making new migrations, you'll have to configure your username and email for sqitch as so:
* `sqitch config --user user.email 'example@example.com'`
* `sqitch config --user user.name 'example'`

To test with Postgres, you will probably want to have
[Docker](https://docs.docker.com/engine/install/)
or [Podman](https://podman.io/docs/installation)
set up on your system.

To run the complete demo, you will have to install
[Minikube](https://minikube.sigs.k8s.io/docs/start/)
and [kubectl](https://kubernetes.io/docs/tasks/tools/#kubectl).
