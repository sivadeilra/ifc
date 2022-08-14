#pragma once

#ifndef __real_stuff_h
#define __real_stuff_h

void do_something_int(int x);

void foo(int x, int y) {
    do_something_int(x);
    do_something_int(y + 1);
}

#endif
