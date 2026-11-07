from pathlib import Path
import sys
sys.path.insert(0, '/home/osso/.worktrees/asset-receipts-proof')
from pinned_godot import pinned_godot
import os, subprocess, time

available = next(int(line.split()[1]) for line in Path('/proc/meminfo').read_text().splitlines() if line.startswith('MemAvailable:'))
if available < 6 * 1024 * 1024:
    raise RuntimeError('Refusing to start server: available RAM below 6 GiB')
base = Path('/home/osso/.worktrees/game-engine-s9/voidscar-proof-4')
base.mkdir(parents=True, exist_ok=True)
server_dir = Path('/home/osso/.worktrees/game-server-fps120')
server_bin = Path('/home/osso/.worktrees/asset-receipts-proof/server-owner/game-server')
admin = Path('/home/osso/.worktrees/asset-receipts-proof/runtime-map0-5fe2/server-bin/game-server-admin')
port = '62870'
socket = '/tmp/game-server-admin-62870.sock'
env = dict(os.environ, GAME_SERVER_PORT=port, GAME_SERVER_BIND_IP='127.0.0.1', GAME_SERVER_DB_PATH=str(base/'game.redb'), GAME_SERVER_ADMIN_SOCKET=socket, GAME_SERVER_GROUND_DIR='/home/osso/.worktrees/game-engine-s9/data')
if (base/'game.redb').exists():
    raise RuntimeError('refusing to reuse database')
with (base/'server.log').open('w') as log:
    server = subprocess.Popen([str(server_bin)], cwd=server_dir, env=env, stdout=log, stderr=subprocess.STDOUT)
    x = None
    try:
        for _ in range(120):
            if server.poll() is not None:
                raise RuntimeError('server exited')
            if subprocess.run([str(admin),'ping'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode == 0:
                break
            time.sleep(1)
        else:
            raise RuntimeError('server startup timeout')
        sockets = subprocess.run(['ss','-ulnp'],capture_output=True,text=True,check=True).stdout
        if '127.0.0.1:'+port not in sockets or 'pid='+str(server.pid)+',' not in sockets:
            raise RuntimeError('server port ownership mismatch')
        for command in [('create-account','fb_receipts','fbtest'),('create-character','fb_receipts','Fbreceipts','10','1'),('teleport','Fbreceipts','2923','4423.14','-451.9','-120.02087276742807')]:
            subprocess.run([str(admin),*command], env=env,check=True)
        x = subprocess.Popen(['Xvfb',':210','-screen','0','1920x1080x24','-nolisten','tcp'], stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
        time.sleep(1)
        if x.poll() is not None:
            raise RuntimeError('Xvfb failed: '+x.stderr.read().decode())
        env.update(DISPLAY=':210',GODOT_TEST_SERVER='127.0.0.1:'+port,BENCH_ACCOUNT='fb_receipts',BENCH_CHARACTER='Fbreceipts',BENCH_SCREENSHOT=str(base/'voidscar'),XDG_CONFIG_HOME=str(base/'xdg'),GAME_ENGINE_ASSET_MODE='extracted-only',ASSET_RESOLVER_DATA_DIR='/home/osso/.worktrees/game-engine-s9/data',ASSET_RESOLVER_SHARED_DATA_DIR='/home/osso/.worktrees/game-engine-s9/data')
        env.pop('WAYLAND_DISPLAY',None)
        cmd=[pinned_godot(),'--display-driver','x11','--rendering-driver','vulkan','--audio-driver','Dummy','--resolution','1920x1080','--path','/home/osso/.worktrees/game-engine-s9/godot','-s','res://tests/receipt_city_probe.gd','--','--server','127.0.0.1:'+port]
        with (base/'probe.log').open('w') as output:
            result=subprocess.run(cmd,env=env,cwd='/home/osso/.worktrees/game-engine-s9',stdout=output,stderr=subprocess.STDOUT,timeout=1500)
        (base/'probe.exit').write_text(str(result.returncode)+'\n')
        print('PROBE_EXIT',result.returncode,flush=True)
    finally:
        if x is not None and x.poll() is None:
            x.terminate(); x.wait(timeout=10)
        server.terminate()
        try: server.wait(timeout=20)
        except subprocess.TimeoutExpired:
            server.kill(); server.wait()
