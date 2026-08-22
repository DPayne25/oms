# OMS

A local-first *order management system (OMS)* that takes a single trade intent and executes it
across multiple broker accounts concurrently, with position size calculated
per-account from a user defined risk configuration.

## Problem

To execute a single trading idea while managing multiple foriegn exchange trading accounts, it can be tedious to login/logout, switch trading platforms, and recalculate risk for each account. This bottleneck leads to:

- delays in execution across all accounts
- missed managment opportunities on a number of accounts
- wasted journaling time for data variance across multiple brokers.

## Solution

OMS solves this problem by having one UI that allows for the execution across all managed accounts at the same time (concurrently).

<img src="attachments\oms-demo.gif" alt="oms-demo">

Developing...
