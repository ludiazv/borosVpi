#! /usr/bin/env python

try:
    # For Python 3.0 and later
    from urllib.request import urlopen
except ImportError:
    # Fall back to Python 2's urllib2
    from urllib2 import urlopen

import argparse
import json
import errno    
import os
import tarfile
import shutil
import platform

#import ssl
#ssl._create_default_https_context = ssl._create_unverified_context

# No windows yet
OSES=["linux_x86_64","darwin_x86_64"]
PIO_MAN="https://dl.platformio.org/packages/manifest.json"
OPENOCD_REPO="https://api.github.com/repos/gnu-mcu-eclipse/openocd/releases/latest"
#SDCC_URL= { "darwin_x86_64" : "http://netix.dl.sourceforge.net/project/sdcc/sdcc-macos-amd64/4.0.0/sdcc-4.0.0-x86_64-apple-macosx.tar.bz2" , 
#            "linux_x86_64" : "http://netcologne.dl.sourceforge.net/project/sdcc/sdcc-linux-amd64/4.0.0/sdcc-4.0.0-amd64-unknown-linux2.5.tar.bz2" 
#}

SDCC_URL= { "darwin_x86_64" : "https://sourceforge.net/projects/sdcc/files/sdcc-macos-amd64/4.0.0/sdcc-4.0.0-x86_64-apple-macosx.tar.bz2" , 
            "linux_x86_64" : "https://sourceforge.net/projects/sdcc/files/sdcc-linux-amd64/4.0.0/sdcc-4.0.0-amd64-unknown-linux2.5.tar.bz2" 
}



# Tools =( tool , min version)
# TOOLS=[("tool-stm8binutils","0.230.0"),("toolchain-sdcc","1.30804.10766"),("tool-stm8tools","0.40.181218"),]
TOOLS=[("tool-stm8binutils","0.230.0"),("tool-stm8tools","0.40.181218"),]
# Dirs
TOOL_DIR="toolchain"
build_only=False

def mkdir_p(path):
    try:
        os.makedirs(path)
    except OSError as exc:  # Python >2.5
        if exc.errno == errno.EEXIST and os.path.isdir(path):
            pass
        else:
            raise

def install_package(name,os_dir,url):
    mkdir_p(TOOL_DIR)
    t_dir= os.path.join(TOOL_DIR,os_dir)
    mkdir_p(t_dir)
    mkdir_p(os.path.join(t_dir, name))
    dest_dir =os.path.join(t_dir,name)
    dest= os.path.join(t_dir,os.path.basename(url))
    print("Installing %s from %s ...." % (name,url))
    
    f = urlopen(url)
    with open(dest, "wb") as local_file:
            local_file.write(f.read())
    
    print("Download done! ... Unpacking ...")
    if (dest.endswith("tar.gz") or dest.endswith(".tgz")):
        tar = tarfile.open(dest, "r:gz")
        tar.extractall(dest_dir)
        tar.close()
    elif (dest.endswith("tar.bz2") or dest.endswith(".tbz2")):
        tar = tarfile.open(dest,"r:bz2")
        tar.extractall(dest_dir)
        tar.close()
    elif (dest.endswith("tar")):
        tar = tarfile.open(dest, "r:")
        tar.extractall(dest_dir)
        tar.close()

    os.remove(dest)
    print("Installed %s in %s" % (name,dest_dir))
    


def install(dos):
    dd = os.path.join(TOOL_DIR,dos)
    if(os.path.isdir(dd)):
        print("toolchain detected removing before install... skiping ... please run uninstall if you want to clean toolchain")
        return
        #uninstall()

    print("Download SDDC from SDDC repository...")
    url=SDCC_URL[dos]
    if url is not None:
        install_package("tmp",".",url)
        src=os.path.join(TOOL_DIR,"tmp")
        sdccdir=os.listdir(src)[0]
        shutil.move(os.path.join(src,sdccdir),os.path.join(TOOL_DIR,dos,"toolchain-sdcc"))
        shutil.rmtree(os.path.join(TOOL_DIR,"tmp"))
    else:
        print("SDDC from source not supported for %s" % (dos)) 
        return
    

    print("Download PIO manifest for other tools....")
    manr= urlopen(PIO_MAN)
    try:
        m = json.loads(manr.read().decode(manr.info().get_param('charset') or 'utf-8'))
    except:
        m = json.load(manr)

    manr.close()
    
    print("Processing manifest....")
    for tool in TOOLS:
        t,v = tool
        for ver in m[t]:
            if( dos in ver['system'] and ver['version'] >= v):
                install_package(t,dos,ver['url'])
    
    if build_only:
        return

    print("Open OCD GNU")
    manr=urlopen(OPENOCD_REPO)
    try:
        m = json.loads(manr.read().decode(manr.info().get_param('charset') or 'utf-8'))
    except:
        m = json.load(manr)
    manr.close()

    fdos=""
    if "darwin" in dos:
        fdos="macos.tgz"
    elif "linux" in dos:
        if "i686" in dos:
            fdos="centos32.tgz"
        else:
            fdos="centos64.tgz"

    for a in m['assets']:
        if a['content_type']== "application/x-gzip" and a['name'].endswith(fdos):
            install_package("tmp",".",a['browser_download_url'])
            # path dir structure
            src=os.path.join(TOOL_DIR,"tmp","gnu-mcu-eclipse","openocd")
            ocddir=os.listdir(src)[0]
            shutil.move(os.path.join(src,ocddir),os.path.join(TOOL_DIR,dos,"tool-openocd"))
            shutil.rmtree(os.path.join(TOOL_DIR,"tmp"))



def uninstall():
    print("Remove toolchaing in %s" %(TOOL_DIR))
    shutil.rmtree(TOOL_DIR,ignore_errors=True)

if __name__== "__main__":
    def_os=platform.system().lower() + "_" + platform.machine().lower() 
    parser = argparse.ArgumentParser(description="STM8 Toolchain manager")
    parser.add_argument("action",help="install or uninstall all toolchains",choices=["install","uninstall"])
    parser.add_argument("-o","--os",choices=OSES,help="operating system")
    parser.add_argument("-b","--build-only", action="store_true", help="Not install flash and debug tools")
    p=parser.parse_args()
    
    print(p)
    if p.os is not None:
        def_os = p.os

    # Remove last tool if no debug is wanted    
    if p.build_only:
        print("Build tools only")
        build_only=True
        del TOOLS[-1]
    
    print("Using os: %s" %(def_os))
    if p.action == "install":
        install(def_os)
    else:
        uninstall()
    
    print("done!")
