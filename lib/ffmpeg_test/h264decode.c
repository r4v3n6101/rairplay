#include <stdio.h>

#include <libavcodec/avcodec.h>

static char* OUTPUT_FILE = "h264.raw";

void append_frame_to_file(const AVFrame* avframe) {
    FILE* file = fopen(OUTPUT_FILE, "ab");
    if (!file) {
        fprintf(stderr, "Could not open file");
        return;
    }

    for (int y = 0; y < avframe->height; y++) {
        fwrite(avframe->data[0] + y * avframe->linesize[0], 1, avframe->width, file);
    }

    for (int y = 0; y < avframe->height / 2; y++) {
        fwrite(avframe->data[1] + y * avframe->linesize[1], 1, avframe->width / 2, file);
    }

    for (int y = 0; y < avframe->height / 2; y++) {
        fwrite(avframe->data[2] + y * avframe->linesize[2], 1, avframe->width / 2, file);
    }

    fclose(file);
}

extern void decode_frame(AVCodecContext* avctx, const void* pkt, int pktlen)
{
    int ret;
    AVFrame* avframe = av_frame_alloc();
    AVPacket* avpkt = av_packet_alloc();
    avpkt->data = av_malloc(pktlen + AV_INPUT_BUFFER_PADDING_SIZE);
    avpkt->size = pktlen;
    memcpy(avpkt->data, pkt, pktlen);

    ret = avcodec_send_packet(avctx, avpkt);
    if (ret < 0) {
        fprintf(stderr, "Can't send packet: %s\n", av_err2str(ret));
        goto clear;
    }

    while (1) {
        ret = avcodec_receive_frame(avctx, avframe);
        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            goto clear;
        } else if (ret < 0) {
            fprintf(stderr, "Can't receive frame: %s\n", av_err2str(ret));
            goto clear;
        }

        append_frame_to_file(avframe);
        static int frame_count = 0;
        fprintf(stdout, "[%d] Frame written: width=%d, height=%d\n", ++frame_count, avframe->width, avframe->height);
    }

    clear:
        av_packet_free(&avpkt);
        av_frame_free(&avframe);
}

extern AVCodecContext* init_ctx(const void* avcc, int avcclen)
{
    AVCodecContext* avctx = avcodec_alloc_context3(NULL);

    AVCodecParameters* params = avcodec_parameters_alloc();
    params->extradata_size = avcclen;
    params->extradata = av_malloc(avcclen + AV_INPUT_BUFFER_PADDING_SIZE);
    memcpy(params->extradata, avcc, avcclen);
    avcodec_parameters_to_context(avctx, params);
    avcodec_parameters_free(&params);

    const AVCodec* codec = avcodec_find_decoder(AV_CODEC_ID_H264);
    avcodec_open2(avctx, codec, NULL);

    return avctx;
}

extern void free_ctx(AVCodecContext** avctx) {
    avcodec_free_context(avctx);
}
