port module Main exposing (main)

import Platform

port emitResult : Int -> Cmd msg

fib : Int -> Int
fib n =
    if n < 2 then
        n
    else
        fib (n - 1) + fib (n - 2)

main : Program () () msg
main =
    Platform.worker
        { init = \_ -> ( (), emitResult (fib 30) )
        , update = \_ model -> ( model, Cmd.none )
        , subscriptions = \_ -> Sub.none
        }
