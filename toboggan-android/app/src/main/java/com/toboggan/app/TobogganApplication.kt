package com.toboggan.app

import android.app.Application

class TobogganApplication : Application() {

    override fun onCreate() {
        super.onCreate()
        // Load the native Rust library
        System.loadLibrary("toboggan")
    }
}
