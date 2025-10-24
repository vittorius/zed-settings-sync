use notify::{Event, RecursiveMode, Result as NotifyResult, Watcher};
use std::{error::Error, path::Path, sync::mpsc};
use {
    interprocess::local_socket::{GenericNamespaced, ListenerOptions, Stream, prelude::*},
    std::io::{self, BufReader, prelude::*},
};

fn main() -> Result<(), Box<dyn Error>> {
    // FIXME: ensure that only one instance of this process can run
     
    // Define a function that checks for errors in incoming connections. We'll use this to filter
    // through connections that fail on initialization for one reason or another.
    fn handle_error(conn: io::Result<Stream>) -> Option<Stream> {
        match conn {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("Incoming connection failed: {e}");
                None
            }
        }
    }

    // Pick a name.
    let printname = "zed-settings-sync-watcher.sock";
    let name = printname.to_ns_name::<GenericNamespaced>()?;

    // Configure our listener...
    let opts = ListenerOptions::new().name(name);

    // ...then create it.
    let listener = match opts.create_sync() {
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            // When a program that uses a file-type socket name terminates its socket server
            // without deleting the file, a "corpse socket" remains, which can neither be
            // connected to nor reused by a new listener. Normally, Interprocess takes care of
            // this on affected platforms by deleting the socket file when the listener is
            // dropped. (This is vulnerable to all sorts of races and thus can be disabled.)
            //
            // There are multiple ways this error can be handled, if it occurs, but when the
            // listener only comes from Interprocess, it can be assumed that its previous instance
            // either has crashed or simply hasn't exited yet. In this example, we leave cleanup
            // up to the user, but in a real application, you usually don't want to do that.
            eprintln!(
                "Error: could not start server because the socket file is occupied. Please check
                if {printname} is in use by another process and try again."
            );
            return Err(Box::new(e)); // TODO: switch to anyhow
        }
        x => x?,
    };

    // The syncronization between the server and client, if any is used, goes here.
    eprintln!("Server running at {printname}");

    // Preemptively allocate a sizeable buffer for receiving at a later moment. This size should
    // be enough and should be easy to find for the allocator. Since we only have one concurrent
    // client, there's no need to reallocate the buffer repeatedly.
    let mut buffer = String::with_capacity(128);

    for conn in listener.incoming().filter_map(handle_error) {
        // Wrap the connection into a buffered receiver right away
        // so that we could receive a single line from it.
        let mut conn = BufReader::new(conn);

        conn.read_line(&mut buffer)?;

        print!("workspace path received: {buffer}");
        
        watch_workspace(Path::new(&buffer))?;

        // Clear the buffer so that the next iteration will display new data instead of messages
        // stacking on top of one another.
        buffer.clear();
    }
    //{
    Ok(())
} //}

fn watch_workspace(workspace_path: &Path) -> NotifyResult<()> {
    let (tx, rx) = mpsc::channel::<NotifyResult<Event>>();
    
        // Use recommended_watcher() to automatically select the best implementation
        // for your platform. The `EventHandler` passed to this constructor can be a
        // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
        // another type the trait is implemented for.
        let mut watcher = notify::recommended_watcher(tx)?;
    
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(workspace_path, RecursiveMode::Recursive)?;
        // Block forever, printing out events as they come in
        for res in rx {
            match res {
                Ok(event) => println!("event: {event:?}"),
                Err(e) => println!("watch error: {e:?}"),
            }
        }
    
        Ok(()) 
}
