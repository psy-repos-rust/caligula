{
  config,
  pkgs,
  lib,
  modulesPath,
  ...
}:
let
  gdbport = 12345;
in
{
  imports = [
    "${modulesPath}/profiles/minimal.nix"
    "${modulesPath}/profiles/qemu-guest.nix"

    # Needed so that the QEMU options exist
    "${modulesPath}/virtualisation/qemu-vm.nix"
  ];

  networking.useDHCP = true;
  networking.firewall.enable = false;

  # Automatically log in as development user
  services.getty.autologinUser = "incitatus";
  users.users.incitatus = {
    isNormalUser = true;
    password = "";
    extraGroups = [ "wheel" ];
  };

  # Ensure root doesn't need a password
  security.sudo.wheelNeedsPassword = false;
  users.users.root.password = "";

  system.stateVersion = "25.11";

  # Run a GDB server
  systemd.services.gdbserver = {
    description = "GDB server";
    wantedBy = [ "multi-user.target" ];
    path = [ pkgs.gdb ];
    script = ''
      gdbserver --multi 0.0.0.0:${builtins.toString gdbport}
    '';

    serviceConfig = {
      Type = "simple";
      User = "root";
      Restart = "always";
      RestartSec = 5;
    };
  };

  virtualisation = {
    mountHostNixStore = true;
    useBootLoader = false;

    sharedDirectories.caligula = {
      source = ''"$CALIGULA_DIR"'';
      target = "/caligula";
    };

    qemu.options = [
      # Expose host's CPU to guest as normal
      "-cpu host"

      # Expose VM's monitor console to a socket
      ''-monitor unix:"$CALIGULA_DIR"/devvm.sock,server,nowait''

      # Needed or else ctrl-c kills the VM
      "-serial mon:stdio"

      # Create a USB bus named xhci. We will be sticking devices
      # onto this for testing purposes.
      "-device nec-usb-xhci,id=xhci"
    ];

    # Expose the GDB server to the host via socket
    qemu.networkingOptions = lib.mkForce [
      ''-nic user,hostfwd=unix:"$CALIGULA_DIR"/devvm_gdb.sock-:${builtins.toString gdbport}''
    ];

    cores = 8;
    memorySize = 2048; # MiB

    # Don't make a disk image. The VM should run off a tmpfs.
    diskImage = null;

    # Make it run directly in the console
    graphics = false;
  };

}
