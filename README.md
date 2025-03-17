
## librubicon

This tool serves as partial Rust reimplementaion of [auto_inject](https://github.com/DataDog/auto_inject/) - with additional capability of being able to restart users applications on Demand and with updated configuraiton


See early [demo](https://drive.google.com/file/d/1_W_1cfe4v_0RDSz4l30FCjZRZepH_9Kb/view?usp=sharing) - showing ability to restart a bash process with the ability to change its environment config.
When key=value is written to a file. The Watchdog subprocess will restart the applicaiton

### technical decisions

The initial POC uses LD_PRELOAD mechanism to modify the existing process on the fly - without having to change any execution patterns manually.

Additionally this library avoids the use of any libc funcitons - in fact its compiled using two distinct and new Rust targets (one for arm64 one for x86_64) - that do not link to libc.
And thus can not use Rust `std`. 

Using `no_std` rust has benefits of better safety - and compatibility with different libc implementations. And also portability - this library will work on any platform where LD_PRELOAD will work.

### Rubicon name

This solution is quite unorthodox. It means the software can now take over PID 1 repsponsibilities - and become sort of process manager itself. 

This is definitely crossing some lines - but can still be very useful tool in our disposal.
