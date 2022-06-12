import sys
import subprocess
import shlex


def invoke_process_popen_blocking(command, shellType=False, stdoutType=subprocess.PIPE):
    """runs subprocess with Popen, but output only returned when process complete"""
    try:
        process = subprocess.Popen(
            shlex.split(command), shell=shellType, stdout=stdoutType)
        (stdout, stderr) = process.communicate()
        print(stdout.decode())
    except:
        print("ERROR {} while running {}".format(sys.exc_info()[1], command))


def invoke_process_popen_poll_live(command, shellType=False, stdoutType=subprocess.PIPE):
    """runs subprocess with Popen/poll so that live stdout is shown"""
    try:
        process = subprocess.Popen(
            shlex.split(command), shell=shellType, stdout=stdoutType)
    except:
        print("ERROR {} while running {}".format(sys.exc_info()[1], command))
        return None
    while True:
        read_input_string = process.stdout.readline()
        if process.poll() is not None:
            break
        if read_input_string:
            input_number = run_square_live(read_input_string.decode())
            output_string = make_output_string(input_number)
            print(output_string.strip())
    rc = process.poll()
    return rc

def run_square_live(input_string):
    input_string_list = input_string.split(" ")
    if len(input_string) > 1:
        input_number = int(input_string_list[1])
    else:
        input_number = 0
    return input_number**2

def make_output_string(input_number):
    output_string = "iteration squared = {}".format(input_number)
    return output_string

def main(argv):
    while True:

        prompt = "Execute which commmand [./loopWithSleep.sh]: "
        cmd = input(prompt)
        if "quit" == cmd:
            break
        if "" == cmd:
            cmd = "./shell_pipe_writer.sh"

        print("== invoke_process_popen_blocking  ==============")
        invoke_process_popen_blocking(cmd)

        print("== invoke_process_popen_poll_live  ==============")
        invoke_process_popen_poll_live(cmd)


if __name__ == '__main__':
    main(sys.argv)