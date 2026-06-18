import "./index.css";
import { Composition } from "remotion";
import { ThreadVideo, videos } from "./Composition";

export const RemotionRoot = () => {
  return (
    <>
      {videos.map((spec) => (
        <Composition
          key={spec.id}
          id={spec.id}
          component={ThreadVideo}
          durationInFrames={spec.durationInFrames}
          fps={30}
          width={1280}
          height={720}
          defaultProps={{ spec }}
        />
      ))}
    </>
  );
};
