#!/bin/bash

(cd fat_badges; cargo contract build) && \
(cd easy_oracle; cargo contract build) && \
(cd advanced_judger; cargo contract build)
