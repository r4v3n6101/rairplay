#!/bin/sh

ffplay -f rawvideo -pixel_format yuv420p -color_range 2 -video_size 498x1080 -framerate 60 h264.raw
